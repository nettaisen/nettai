--!strict
-- ShadeMan, the navi of the ShadeMan chips (EXE5's actor object #0x19, 0x080BD0E8; EXE5's own): @exelib/shademan/navi,
-- with EXE5's look (his sprite, 08-0D, appearing in animation 3, calling out in 5; his sounds, and one as a command is
-- in) and his crush (./crush); he bows before he goes. His commands (0x080BD194, 0x080BD1A0): down, down or right,
-- right or A paralyzes (without a flinch: the crush's parameters 0x1201, its palette 1); down, down or left, left or A
-- confuses (0x2209, its palette 9); without one the crush carries nothing.

local common = require("@exelib/shademan/navi")
local crush = require("./crush")
local navi_chips = require("@exelib/navi_chips/navi")
local statuses = require("../../rules/status")

local SHADEMAN = common.make {
    id = "shademan/navi",
    sprite = asset.sprite("shademan"),
    appear_anim = 3,
    call_anim = 5,
    appear = asset.sound("appear"),
    call = asset.sound("shademan-call"),
    command_sound = asset.sound("ok-8b"),
    crush = crush,
    commands = {
        {
            steps = { { "down" }, { "down", "right" }, { "right", "a" } },
            hit = { palette = 1, hit_mod = 0, status = statuses.paralyze_150 },
        },
        {
            steps = { { "down" }, { "down", "left" }, { "left", "a" } },
            hit = { palette = 9, hit_mod = 3, status = statuses.confuse_960 },
        },
    },
    uncommanded = function(): ShadeManHit
        return { palette = 0, hit_mod = 3, status = nil }
    end,
    bows = true,
}

local navi = {}

-- 0x080BD288: a chip's ShadeMan for `user`, reporting to `controller` (the summon table's entry 1), in its palette.
function navi.summon(palette: number): NaviChipSummon
    return function(user: Object, controller: Object, spec: NaviChipSpec): Object?
        local o = navi_chips.spawn(SHADEMAN.kind, user, spec)
        if o then
            SHADEMAN.summoned(o, controller, palette)
        end
        return o
    end
end

return navi
