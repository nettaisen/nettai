--!strict
-- SearchMan, the navi of EXE4's SearchMan chips (EXE4's actor object #0x31, 0x080C355C), whom the shared navi chip
-- controller (objects/navi_chip_controller, its entry 0) brings: @exelib/serchman/navi with EXE4's look (his sprite,
-- 08-04, in his chip's palette, 0x0800B950: two palettes a variant; his rifle up in animation 15 and a shot in 10; the
-- shot's sound 0xBB) and his spawner (0x080C37D4: neither his user, nor its facing, nor the element; his height his
-- side); his scope (./scope) reads his side's navi's presses.

local common = require("@exelib/serchman/navi")
local navi_chips = require("../../lib/navi_chips")
local scope = require("./scope")
local shot = require("./shot")

-- His palettes a variant (0x0800B9B1's entry 4).
local PALETTES = 2

local searchman = common.make {
    id = "serchman/navi",
    sprite = asset.sprite("searchman"),
    appear_sound = asset.sound("appear"),
    shot_sound = asset.sound("vulcan"),
    palettes = {
        [0] = navi_chips.palette(PALETTES, 0),
        [3] = navi_chips.palette(PALETTES, 3),
        [4] = navi_chips.palette(PALETTES, 4),
    },
    aim_anim = 15,
    shot_anim = 10,
    scope = scope,
    shot = shot,
    summoned_alone = true,
}

local summons: { [number]: (user: Object, controller: Object, spec: NaviChipSpec) -> Object? } =
    { [0] = searchman.summon_at(0), [3] = searchman.summon_at(3), [4] = searchman.summon_at(4) }

-- 0x080C37D4: SearchMan for the controller's user, with the damage plus the bonus, of the chip's level.
local function spawn(controller: Object, user: Object, damage: number): Object?
    local level = (controller.state :: NaviChipControllerState).level
    local summon = summons[level]
    if not summon then
        error(string.format("SearchMan's controller has a level %d no chip gives", level))
    end
    return summon(
        user,
        controller,
        { panel_x = controller.panel_x, panel_y = controller.panel_y, element = controller.element, damage = damage }
    )
end

return { kind = searchman.kind, spawn = spawn }
