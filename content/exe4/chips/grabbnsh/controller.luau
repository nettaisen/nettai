--!strict
-- EXE4's GrabRvng's and GrabBnsh's dimming controller (effect object #0x22, 0x080E474C) and hand (attack object #0x46,
-- 0x080D5FF8): @exelib/grabbnsh (EXE6's and EXE5's code) with EXE4's look and its own taking back. The hand drops from
-- 256 pixels on the grab shot's sheet (0c-13) with its sounds (0x93, 0x94) and hits as a thrown break of hit modifier
-- 0x0B. The taking back (0x080E4836) counts the panels of the user's home columns (side 0's 1 to 3, side 1's 4 to 6)
-- the other side holds, each flashing (effect row 0x45: the grab shot's animation 1), and returns none; with its sound
-- (0x79) a hand drops on every enemy navi for each, 31 ticks apart; then 91 ticks (set as the strikes end).

local collision = require("../../rules/collision")
local common = require("@exelib/grabbnsh/controller")
local hand = require("@exelib/grabbnsh/hand")

local SHEET = asset.sprite("grab-shot")
local FLASH = new.effect { sprite = SHEET, anim = 1, palette = 0 }
-- The other side's ownership of a panel, by the user's side (0x080E4888: side 1's bit 0x20, set or clear).
local OTHER = { [0] = { require = 0x20, forbid = 0 }, [1] = { require = 0, forbid = 0x20 } }
local AFTER_STRIKES = 90

local hand_kind = hand.make_kind {
    id = "grabbnsh/hand",
    sprite = SHEET,
    fall_sound = asset.sound("grab-shot-2"),
    land_sound = asset.sound("grab-shot"),
    target = collision.hits_navis,
    self_type = collision.thrown_break,
    height = 0x100_0000,
    hit_mod = 0x0B,
}

-- 0x080E4836: of the user's home columns (from 0x080E4898's, by side: 1 or 4), each panel, the rows from the bottom,
-- that the other side holds flashes; how many.
local function take_back(me: Object): number
    local other = OTHER[me.alliance]
    local first = 1 + 3 * me.alliance
    local n = 0
    for x = first, first + 2 do
        for y = 3, 1, -1 do
            if field.check(x, y, other.require, other.forbid) then
                n += 1
                local cx, cy = field.center(x, y)
                battle.effect(Vec3.new(cx, cy, 0), FLASH)
            end
        end
    end
    return n
end

return common.make {
    id = "grabbnsh/controller",
    hand = hand_kind,
    returned = FLASH,
    return_sound = asset.sound("program-advance-part"),
    take_back = take_back,
    after_strikes = AFTER_STRIKES,
}
