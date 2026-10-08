--!strict
-- The NaviCust bug a hit inflicts on EXE4's navis (0x0800D9E8, EXE6's `sub_80139F6`): a hit's bug code and argument
-- (its collision's `bugs`, the low and the high byte) reach the navi it hits (`inflicted_bugs`). Code 0xFF raises the
-- side's NaviCust bug the argument numbers (0x0804770C: ./init's `raise_bug`); any other code names the NaviStats byte
-- the argument goes to (the stats by their names here). Then the navi's abilities come back from its stats (0x0800D906,
-- EXE6's `sub_801393A`: the engine's, on the answer "edited").
--
-- The codes EXE4's objects set: ShadeMan's crush, 0x0D (the move bug, its argument a direction drawn at his init);
-- RedSun's stripping hits (attack 0x87, 0x080DE284's words), 1 to 7 and 0x0A; attack 0x41's (0x080D5880's words,
-- through effect 0x1B), those and 0x0C and 0x12; weapon routine 0x5C's hit (0x080F29C6), 0x0E; BugBomb's (attack
-- 0x77, 0x080DC134: kinds 1, 2, 3, 4, 6, 8) and BugCurse's (attack 0x80, 0x080DD494: kinds 2, 3, 4, 6), 0xFF. The
-- original takes any other code to the NaviStats byte it numbers too; no object sets one.

local navicust = require("./init")

local bugs = {}

-- The NaviCust bugs by the number a code 0xFF's argument gives (0x08042B18's rows: 8, the weapon's, and 11, which does
-- nothing, are no program's here), as their names.
local function bug_named(kind: number): string?
    if kind == 1 then
        return "move"
    elseif kind == 2 then
        return "hp"
    elseif kind == 3 then
        return "custom"
    elseif kind == 4 then
        return "panel"
    elseif kind == 5 then
        return "encounter"
    elseif kind == 6 then
        return "buster"
    elseif kind == 7 then
        return "support"
    elseif kind == 9 then
        return "result"
    elseif kind == 10 then
        return "hub"
    end
    return nil
end

-- 0x0800D9E8's part: the navi's hit's code, if any, written; "edited" then.
function bugs.navi_bug(side: number, navi: Object): ("edited" | "spared")?
    local word = navi.collision.inflicted_bugs
    local code, arg = bit32.band(word, 0xFF), bit32.band(bit32.rshift(word, 8), 0xFF)
    if code == 0 then
        return nil
    end
    local s = battle.navi(side)
    if code == 0xFF then
        local bug = bug_named(arg)
        if bug then
            navicust.raise_bug(side, bug)
        elseif arg == 8 then
            -- (Its levels set the charged shot to weapon routines 0x1F, 0x20 and 0x21, of which the content has 0x21,
            -- DrkCanon's taunt, alone.)
            error("a hit's bug raises the NaviCust's weapon bug (8): its weapon routines 0x1F and 0x20 aren't content")
        elseif arg ~= 11 then
            error(string.format("a hit's bug raises NaviCust bug %d, which reads past 0x08042B18's table", arg))
        end
    elseif code == 0x01 then
        s.super_armor = arg ~= 0
    elseif code == 0x02 then
        s.float_shoes = arg ~= 0
    elseif code == 0x03 then
        s.air_shoes = arg ~= 0
    elseif code == 0x04 then
        s.undershirt = arg ~= 0
    elseif code == 0x05 then
        s.attack = arg
    elseif code == 0x06 then
        s.rapid = arg
    elseif code == 0x07 then
        s.charge = arg
    elseif code == 0x0A then
        -- The charged shot's routine: 1, the navi's own (RedSun's, attack 0x41's).
        if arg ~= 1 then
            error(string.format("a hit's bug sets the charged shot to weapon routine %#x: no object sets it", arg))
        end
        local own = s.navi.weapons
        s.charge_shot_weapon = if own then own.charge_shot else nil
    elseif code == 0x0C then
        -- B+Left's routine: 0xFF, none (attack 0x41's).
        if arg ~= 0xFF then
            error(string.format("a hit's bug sets B+Left to weapon routine %#x: no object sets it", arg))
        end
        s.back_special_weapon = nil
    elseif code == 0x0D then
        s.move_bug = arg
    elseif code == 0x0E then
        s.hp_drain = arg
    elseif code == 0x12 then
        s.custom_level = arg
    else
        error(string.format("a hit's bug code %#x: no EXE4 object sets it", code))
    end
    return "edited"
end

return bugs
