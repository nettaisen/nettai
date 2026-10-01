--!strict
-- GreatYo (YoYo's Program Advance): YoYo's throw (chips/yoyo/throw.luau)
-- of a controller that sends three yoyos, one on each row, which roll out
-- four panels, spin for five rounds and move together.
--
-- Its record stays the pack's for now (a Program Advance): this is its
-- action, which registration by number runs (chips/013-yoyo,
-- docs/design/content-model-v2.md §12).

local throw = require("../yoyo/throw")

return {
    action = throw.action { id = "greatyo/action", yoyo = { great = true, panels = 4, rounds = 5 } },
}
