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

return widget
