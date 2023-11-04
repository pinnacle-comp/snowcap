require("snowcap").setup(function(snowcap)
	snowcap.widget.new(100, 250, "TopLeft", {
		Slider = {
			range_start = 0.0,
			range_end = 1.0,
			width = "Fill",
			height = 30,
			step = 0.1,
			on_change = 1,
		},
	})
end)
