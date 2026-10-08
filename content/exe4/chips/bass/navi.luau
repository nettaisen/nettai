--!strict
-- Bass, the navi of EXE4's Bass chip (EXE4's actor object #0x4F, 0x080CADC0): EXE6's and EXE5's (@exelib/bass/navi;
-- the same code but for his sprite, 08-19, his cape's animation offset 0x0E, his wind-up's and volleys' animations 3
-- and 4, his last volley's 5, whose end ends him) with EXE4's shot (./shot), his shots' flash (effect row 0x6D, its
-- jitter 0x1F: 0x080CB0D0) and his smoke (effect row 0x13). His controller (effect object #0x76, 0x080EA4C8) runs
-- lib/navi_chips' short course.

local common = require("@exelib/bass/navi")
local navi_chips = require("../../lib/navi_chips")
local shot = require("./shot")

local BASS = common.make {
    id = "bass/navi",
    sprite = asset.sprite("bass"),
    cape_anim_offset = 0x0E,
    raise_anim = 3,
    volley_anim = 4,
    ending = "animation",
    last_anim = 5,
    shoot = function(me: Object, x: number, y: number, damage: number)
        shot.spawn(me, x, y, damage)
    end,
    flash = new.effect { sprite = asset.sprite("hit-sparks"), anim = 8, palette = 0 },
    flash_jitter = 0x1F,
    smoke = new.effect { sprite = asset.sprite("puff"), anim = 0, palette = 0 },
    appear = asset.sound("appear"),
}

-- What the Bass chip states for its own controller: the controller's id (the original's effect object #0x76).
export type ChipSpec = { id: string }

-- The Bass chip's controller: the short course (the user warps out, 11 ticks, Bass, 21 ticks after he left, the user
-- warps back in). (0x080CAF68: Bass for the controller's user, with the damage plus the bonus.)
local function controller(chip: ChipSpec): NaviChipController
    return navi_chips.controller {
        id = chip.id,
        name = "Bass",
        spawn = function(c: Object, user: Object, damage: number): Object?
            return BASS.summon(user, c, { panel_x = c.panel_x, panel_y = c.panel_y, element = c.element, damage = damage })
        end,
        short = navi_chips.SHORT,
    }
end

return { controller = controller }
