--!strict
-- BassAnly, the navi of EXE4's BassAnly chip (EXE4's actor object #0x50, 0x080CB0FC): EXE6's and EXE5's
-- (@exelib/bassanly/navi; EXE5's code without his arm) with EXE4's parts: Bass's sheet 08-19, his cloak on his
-- animation + 0x0E (0x080CB184: 0x0E011908), EXE5's animations (9 his arm rising, 10 up, 11 firing, 12 its loop, 13
-- after), the glow in his hands as his arm is up (effect row 0x58: his sheet's animation 32, for 26 ticks: 0x080CB236),
-- his smoke (effect row 0x13) and EXE4's dark ball (./shot). His controller (effect object #0x77, 0x080EA598) runs
-- lib/navi_chips' short course.

local common = require("@exelib/bassanly/navi")
local navi_chips = require("../../lib/navi_chips")
local shot = require("./shot")

local SPRITE = asset.sprite("bass")

local BASSANLY = common.make {
    id = "bassanly/navi",
    sprite = SPRITE,
    cloak = { sprite = SPRITE, stepping = "while_dimmed", anim_offset = 0x0E },
    sounds = { appear = asset.sound("appear"), raise = asset.sound("panel-hole") },
    anims = { raise = 9, stance = 0xA, fire = 0xB, fire_loop = 0xC, rest = 0xD },
    glow = { effect = new.effect { sprite = SPRITE, anim = 32, palette = 0 }, ticks = 26 },
    smoke = new.effect { sprite = asset.sprite("puff"), anim = 0, palette = 0 },
    -- His shots (0x080CB2FA): level 2, the fastest, seeking enemy navis along the slant.
    shoot = function(owner: Object, x: number, y: number, slant: number, damage: number): Object?
        return shot.spawn(owner, x, y, 0, { slant = slant, homing = "seeking" }, damage, 0)
    end,
}

-- What the BassAnly chip states for its own controller: the controller's id (the original's effect object #0x77).
export type ChipSpec = { id: string }

-- The BassAnly chip's controller: the short course (the user warps out, 11 ticks, BassAnly, 21 ticks after he left,
-- the user warps back in). (0x080EA608: BassAnly for the controller's user, with the damage plus the bonus.)
local function controller(chip: ChipSpec): NaviChipController
    return navi_chips.controller {
        id = chip.id,
        name = "BassAnly",
        spawn = function(c: Object, user: Object, damage: number): Object?
            local nspec = { panel_x = c.panel_x, panel_y = c.panel_y, element = c.element, damage = damage }
            return BASSANLY.summon(user, c, nspec)
        end,
        short = navi_chips.SHORT,
    }
end

return { controller = controller }
