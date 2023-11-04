---@meta _

---@alias CallbackId integer

---@alias Length
---| "Fill"
---| { FillPortion: integer }
---| "Shrink"
---| { Fixed: float }

---@alias Anchor
---| "Top"
---| "Bottom"
---| "Left"
---| "Right"
---| "TopRight"
---| "TopLeft"
---| "BottomRight"
---| "BottomLeft"

---@alias Alignment
---| "Start"
---| "Center"
---| "End"

---@class (exact) Padding
---@field top float
---@field bottom float
---@field right float
---@field left float

---@alias Msg _Msg

---@class (exact) _Msg
---@field NewWidget NewWidget

---@class (exact) NewWidget
---@field widget WidgetDefinition
---@field width integer
---@field height integer
---@field anchor Anchor

---@class (exact) WidgetDefinition
---@field Slider Slider?
---@field Column Column?
---@field Button Button?
---@field Text Text?

---@class (exact) Slider
---@field range_start float
---@field range_end float
---@field value CallbackId
---@field on_change CallbackId
---@field on_release CallbackId?
---@field width Length
---@field height integer
---@field step float

---@class (exact) Column
---@field spacing integer
---@field padding Padding
---@field width Length
---@field height Length
---@field max_width integer
---@field alignment Alignment
---@field children WidgetDefinition[]

---@class (exact) Button
---@field width Length
---@field height Length
---@field padding Padding
---@field child WidgetDefinition

---@class (exact) Text
---@field text string

----------------------------------------------------------------

---@class (exact) IncomingMsg
---@field CallCallback CallCallback?

---@class (exact) CallCallback
---@field callback_id CallbackId
---@field args Args?

---@class Args
