require("snowcap").setup(function(snowcap)
	local widget = snowcap.widget

	widget.new(
		100,
		250,
		"TopLeft",
		widget.slider({
			range_start = 0.0,
			range_end = 1.0,
			width = "Fill",
			height = 30,
			step = 0.1,
			on_change = function(value)
				print("got value " .. tostring(value))
			end,
		})
	)
end)
