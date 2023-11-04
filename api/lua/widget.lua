---@class Widget
local widget = {}

---Create a new widget.
---@param width integer
---@param height integer
---@param anchor Anchor
---@param definition WidgetDefinition
--TODO: return widget handle
function widget.new(width, height, anchor, definition)
	SendMsg({
		NewWidget = {
			width = width,
			height = height,
			anchor = anchor,
			widget = definition,
		},
	})
end

---@class SliderDefinition
---@field range_start float
---@field range_end float
---@field step float
---@field on_change fun(value: float)
---@field on_release fun()?
---@field width Length
---@field height integer

---Define a slider.
---@param definition SliderDefinition
---@return WidgetDefinition
function widget.slider(definition)
	---@param args Args?
	table.insert(CallbackTable, function(args)
		definition.on_change(args.SliderValue)
	end)
	local on_change_callback_id = #CallbackTable

	local on_release_callback_id = nil
	if definition.on_release then
		table.insert(CallbackTable, function(_)
			definition.on_release()
		end)
		on_release_callback_id = #CallbackTable
	end

	---@type WidgetDefinition
	local slider = {
		Slider = {
			range_start = definition.range_start,
			range_end = definition.range_end,
			step = definition.step,
			width = definition.width,
			height = definition.height,
			on_change = on_change_callback_id,
			on_release = on_release_callback_id,
		},
	}

	return slider
end

return widget
