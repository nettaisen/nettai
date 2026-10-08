--!strict
-- NumberMan, the navi of the NumberMan chips (EXE5's actor object #0x0F, 0x080BB34C; EXE5's own, built on EXE6's navi
-- template): @exelib/numbrman/navi, with EXE5's look (his sprite, 08-09; his sounds), his face (./face) and his die
-- (./dice), which his throw gives 20 HP, no status and hit modifier 3 (0x080BB4C2's word 0x30014). Each chip states
-- his palette (0x0800DA8A: navi 9's count, 1, by 0x0800DAE8's row by the chip's level: 0, an SP's 2, a DS's 3).

local common = require("@exelib/numbrman/navi")
local dice = require("./dice")
local face = require("./face")
local navi_chips = require("@exelib/navi_chips/navi")

local NUMBERMAN = common.make {
    id = "numbrman/navi",
    sprite = asset.sprite("numberman"),
    appear = asset.sound("appear"),
    throw = asset.sound("minibomb-throw"),
    face = face,
    dice = dice,
    die = { hp = 0x14, status = nil, hit_mod = 3 },
}

local navi = {}

-- 0x080BB540: a chip's NumberMan for `user`, reporting to `controller` (the summon table's entry 0), in its palette.
function navi.summon(args: { palette: number }): NaviChipSummon
    return function(user: Object, controller: Object, spec: NaviChipSpec): Object?
        local o = navi_chips.spawn(NUMBERMAN.kind, user, spec)
        if o then
            NUMBERMAN.summoned(o, controller, args.palette)
        end
        return o
    end
end

return navi
