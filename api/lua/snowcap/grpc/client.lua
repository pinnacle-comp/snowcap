-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

require("compat53")

local socket = require("cqueues.socket")
local headers = require("http.headers")
local h2_connection = require("http.h2_connection")
local protobuf = require("pinnacle.grpc.protobuf")
local pb = require("pb")

---@nodoc
---Create appropriate headers for a gRPC request.
---@param service string The desired service
---@param method string The desired method within the service
local function create_request_headers(service, method)
    local req_headers = headers.new()
    req_headers:append(":method", "POST")
    req_headers:append(":scheme", "http")
    req_headers:append(":path", "/" .. service .. "/" .. method)
    req_headers:append("te", "trailers")
    req_headers:append("content-type", "application/grpc")
    return req_headers
end

local function socket_path()
    local dir = os.getenv("XDG_RUNTIME_DIR")
    if not dir then
        print("$XDG_RUNTIME_DIR not set, exiting")
        os.exit(1)
    end

    local wayland_instance = os.getenv("WAYLAND_DISPLAY") or "wayland-0"

    local path = dir .. "/snowcap-grpc-" .. wayland_instance .. ".sock"

    return path
end

local function new_conn()
    local sock = socket.connect({
        path = socket_path(),
    })
    sock:connect()

    local conn = h2_connection.new(sock, "client")
    conn:connect()

    return conn
end

---@class CqueuesLoop
---@field loop function
---@field wrap fun(self: self, fn: function)

---@class H2Connection
---@field new_stream function

---@class H2Stream
---@field write_chunk function
---@field shutdown function

local _conn_success, conn = pcall(new_conn)
if not _conn_success then
    print(
        "Error: failed to connect to Snowcap socket; the next Snowcap API call will crash the program"
    )
end

---@nodoc
---@class Client
---@field conn H2Connection
---@field loop CqueuesLoop
local client = {
    conn = conn,
    loop = require("cqueues").new(),
    version = "v0alpha1",
}

---@class GrpcRequestParams
---@field service string
---@field method string
---@field request_type string
---@field response_type string?
---@field data table

---@nodoc
---Send a synchronous unary request to the compositor.
---
---@param grpc_request_args GrpcRequestArgs
---@param data table
---@return table
function client.unary_request(grpc_request_args, data)
    local stream = client.conn:new_stream()

    local service = grpc_request_args.service
    local method = grpc_request_args.method
    local request_type = grpc_request_args.request
    local response_type = grpc_request_args.response

    local body = protobuf.encode(request_type, data)

    stream:write_headers(create_request_headers(service, method), false)
    stream:write_chunk(body, true)

    -- TODO: check response headers for errors
    local _ = stream:get_headers()

    local response_body = stream:get_next_chunk()

    local trailers = stream:get_headers()
    if trailers then -- idk if im big dummy or not but there are never any trailers
        for name, value, never_index in trailers:each() do
            print(name, value, never_index)
        end
    end

    stream:shutdown()

    -- Skip the 1-byte compressed flag and the 4-byte message length
    ---@diagnostic disable-next-line: redefined-local
    local response_body = response_body:sub(6)
    local response = pb.decode(response_type, response_body)

    return response
end

---@nodoc
---Send a async server streaming request to the compositor.
---
---`callback` will be called with every streamed response.
---
---@param grpc_request_args GrpcRequestArgs
---@param data table
---@param callback fun(response: table)
function client.server_streaming_request(grpc_request_args, data, callback)
    local stream = client.conn:new_stream()

    local service = grpc_request_args.service
    local method = grpc_request_args.method
    local request_type = grpc_request_args.request
    local response_type = grpc_request_args.response

    local body = protobuf.encode(request_type, data)

    stream:write_headers(create_request_headers(service, method), false)
    stream:write_chunk(body, true)

    -- TODO: check response headers for errors
    local _ = stream:get_headers()

    client.loop:wrap(function()
        for response_body in stream:each_chunk() do
            ---@diagnostic disable-next-line: redefined-local
            local response_body = response_body

            while response_body:len() > 0 do
                local msg_len = string.unpack(">I4", response_body:sub(2, 5))

                -- Skip the 1-byte compressed flag and the 4-byte message length
                response_body = response_body:sub(6, 6 + msg_len - 1)

                ---@diagnostic disable-next-line: redefined-local
                local success, obj = pcall(pb.decode, response_type, response_body)
                if not success then
                    print(obj)
                    os.exit(1)
                end

                local response = obj
                callback(response)

                response_body = response_body:sub(msg_len + 1)
            end
        end

        local trailers = stream:get_headers()
        if trailers then
            for name, value, never_index in trailers:each() do
                print(name, value, never_index)
            end
        end
    end)
end

---@nodoc
---@param grpc_request_args GrpcRequestArgs
---@param callback fun(response: table, stream: H2Stream)
---
---@return H2Stream
function client.bidirectional_streaming_request(grpc_request_args, data, callback)
    local stream = client.conn:new_stream()

    local service = grpc_request_args.service
    local method = grpc_request_args.method
    local request_type = grpc_request_args.request
    local response_type = grpc_request_args.response

    local body = protobuf.encode(request_type, data)

    stream:write_headers(create_request_headers(service, method), false)
    stream:write_chunk(body, false)

    -- TODO: check response headers for errors
    local _ = stream:get_headers()

    client.loop:wrap(function()
        for response_body in stream:each_chunk() do
            ---@diagnostic disable-next-line: redefined-local
            local response_body = response_body

            while response_body:len() > 0 do
                local msg_len = string.unpack(">I4", response_body:sub(2, 5))

                -- Skip the 1-byte compressed flag and the 4-byte message length
                response_body = response_body:sub(6, 6 + msg_len - 1)

                ---@diagnostic disable-next-line: redefined-local
                local success, obj = pcall(pb.decode, response_type, response_body)
                if not success then
                    print(obj)
                    os.exit(1)
                end

                local response = obj
                callback(response, stream)

                response_body = response_body:sub(msg_len + 1)
            end
        end

        local trailers = stream:get_headers()
        if trailers then
            for name, value, never_index in trailers:each() do
                print(name, value, never_index)
            end
        end
    end)

    return stream
end

return client
