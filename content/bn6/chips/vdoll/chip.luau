--!strict
-- VDoll: a doll (doll.luau) lobbed three panels ahead, that poisons the
-- panel it lands on and, hurt there, curses the other side with the damage
-- it took (curse.luau). The original's action 0x12 with subtype 8.
--
-- Its record stays the pack's for now (Darkness's Program Advance recipes
-- name it by number): this is its action, which registration by number
-- runs (docs/design/content-model-v2.md §5.1, "As built").

local bomb = require("../../lib/bombs/bomb")
local doll = require("./doll")
local throw = require("../../lib/bombs/throw")

return {
    action = throw.action { id = "vdoll/action", held = bomb.held, thrower = doll.thrower() },
}
