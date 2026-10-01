--!strict
-- YoYo: a yoyo thrown from the panel ahead that rolls out two panels,
-- spins there for two rounds and rolls back (throw.luau).
--
-- Its record stays the pack's for now (GreatYo's Program Advance recipe
-- names it by number): this is its action, which registration by number
-- runs (chips/013-yoyo, docs/design/content-model-v2.md §12).

local throw = require("./throw")

return {
    action = throw.action { id = "yoyo/action", yoyo = { panels = 2, rounds = 2 } },
}
