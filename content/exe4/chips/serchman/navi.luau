--!strict
-- SearchMan, the navi of EXE4's SearchMan chips (EXE4's actor object #0x31, 0x080C355C), and each chip's controller,
-- which brings him: @exelib/serchman/navi with EXE4's look (his sprite, 08-04, in his chip's palette, 0x0800B950: two
-- palettes a variant; his rifle up in animation 15 and a shot in 10; the shot's sound 0xBB) and his spawner
-- (0x080C37D4: neither his user, nor its facing, nor the element; his height his side); his scope (./scope) reads his
-- side's navi's presses.

local common = require("@exelib/serchman/navi")
local navi_chips = require("../../lib/navi_chips")
local scope = require("./scope")
local shot = require("./shot")

local searchman = common.make {
    id = "serchman/navi",
    sprite = asset.sprite("searchman"),
    appear_sound = asset.sound("appear"),
    shot_sound = asset.sound("vulcan"),
    aim_anim = 15,
    shot_anim = 10,
    scope = scope,
    shot = shot,
    summoned_alone = true,
}

-- What each SearchMan chip states for its own controller: the controller's id (the original's effect object #0x42,
-- 0x080E6A18, for all six navis' chips: compat maps each to it), his palette (0x0800B950 by the chip's level: two
-- palettes a variant) and his scope's sweep (0x080E6C78 by the level: 6 ticks a panel, the SP's and DS's 4).
export type ChipSpec = { id: string, palette: number, sweep: number }

-- A SearchMan chip's controller: the user warps out, SearchMan, the user warps back in.
local function controller(chip: ChipSpec): NaviChipController
    local summon = searchman.summon { palette = chip.palette, sweep = chip.sweep }
    -- 0x080C37D4: SearchMan for the controller's user, with the damage plus the bonus.
    local function spawn(c: Object, user: Object, damage: number): Object?
        return summon(user, c, { panel_x = c.panel_x, panel_y = c.panel_y, element = c.element, damage = damage })
    end
    return navi_chips.controller { id = chip.id, name = "SearchMan", spawn = spawn, steps = navi_chips.SHARED }
end

return { controller = controller }
