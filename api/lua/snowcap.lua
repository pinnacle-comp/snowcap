local client = require("snowcap.grpc.client")

---@class snowcap.Snowcap
local snowcap = {
    layer = require("snowcap.layer"),
    widget = require("snowcap.widget"),
}

function snowcap.init()
    require("snowcap.grpc.protobuf").build_protos()
end

function snowcap.listen()
    local success, err = client.loop:loop()
    if not success then
        print(err)
    end
end

---@param setup_fn fun(snowcap: snowcap.Snowcap)
function snowcap.setup(setup_fn)
    snowcap.init()

    setup_fn(snowcap)

    snowcap.listen()
end

return snowcap
