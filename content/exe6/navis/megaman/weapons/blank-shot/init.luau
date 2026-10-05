--!strict
-- The blank shot (`megaman/blank-shot`): the original's weapon routine 2
-- (`sub_8011ADA`) and action 0x33 (`sub_80ED748`): the shared blank shot
-- (@exelib/megaman/blank_shot) with EXE6's buster parts and click
-- (GunDelSol's sound). See docs/engine/objects-and-player.md §B6.

local buster = require("../../../lib/buster")
local common = require("@exelib/megaman/blank_shot")

local blank_shot = {}

local made = common.make { id = "megaman/blank-shot/action", buster = buster, click = asset.sound("gundels1") }

-- `sub_8011ADA`: nothing to fire.
blank_shot.setup = made.setup
blank_shot.action = made.action

blank_shot.weapon = define.weapon {
    id = "megaman/blank-shot",
    -- The charge table's row 2; Charge 5 reads on into row 3's first
    -- entry, as the original's table does.
    charge_ticks = { 180, 180, 180, 180, 180, 40 },
    setup = blank_shot.setup,
}

return blank_shot
