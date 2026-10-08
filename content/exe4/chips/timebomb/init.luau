--!strict
-- TimeBomb (EXE4 chip 0x061): the dimming chips' variant 0x11. Its controller (effect object #0x27, 0x080E4C3C:
-- @exelib/timebom/controller, EXE6's code) sets the countdown bomb (attack object #0x4B, 0x080D6BF8: @exelib/timebom/
-- countdown, EXE6's code's elder, in EXE4's look) where EXE4 places it (0x080E4DE4), with the chip's damage and the
-- telop's bonus, and waits 31 ticks. The bomb counts 3, 2, 1 and blows up the other side's area unless it is broken
-- first.
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local collision = require("../rules/collision")
local controller = require("@exelib/timebom/controller")
local countdown = require("@exelib/timebom/countdown")
local dimming_chips = require("@exelib/dimming")
local panel_bursts = require("../objects/panel_bursts")

-- 0x0801013A: back to its idle, no longer running while paused.
local function return_to_idle(me: Object)
    me.run_while_paused = false
    obstacle.return_to_idle(me)
end

-- The bomb (0x080D6BF8): sheet 0c-23 appearing with 0xB0, ticking with 0xE3 and 0xE4, blowing up with 0xBC over
-- explosions (effect row 0), the panel bursts EXE4's (objects/panel_bursts); a bad panel's puff (row 0x15) as it
-- appears; a puff (0x70, row 0) when broken; no NameID, no tracking, no removal read (0x080D6E6C), its blast even once
-- the battle is over (0x080D6DE8); EXE4's obstacle rows (0x080D6CCC: row 3 does nothing, no frozen or bubbled).
local KIND = countdown.make_kind("timebomb/countdown", {
    sprite = asset.sprite("countdown-bomb"),
    sounds = {
        place = asset.sound("appear"),
        tick = asset.sound("tick"),
        last = asset.sound("last"),
        blast = asset.sound("heat-burst"),
        puff = asset.sound("hit-bomb-1"),
    },
    blast = { is = collision.attack, hits = collision.hits_navis },
    explosion = new.effect { sprite = asset.sprite("explosion"), anim = 0, palette = 0 },
    puff = new.effect { sprite = asset.sprite("puff"), anim = 2, palette = 0 },
    bursts = panel_bursts.kind,
    untracked = true,
    blasts_after_battle = true,
    reads_no_removal = true,
    actions = {
        [1] = return_to_idle,
        [3] = function(_me: Object) end,
        [4] = obstacle.paralyzed,
        [5] = obstacle.knocked_back,
    },
})

-- TimeBomb's bomb (its first parameter 0: 0x080D6C3E's animations 0 and 1, 50 HP): one of its side's field objects
-- (class 1, 0x080D6ECC), a neutral body hit by everything (rows 0x0E and 0x0F).
local BOMB = countdown.variant {
    appear_anim = 0,
    idle_anim = 1,
    hp = 50,
    class = 1,
    collision = { is = collision.neutral_body, hits = collision.everything },
}

-- No bomb where it would go: a puff over the panel, 16 up (effect row 2).
local NO_BOMB = new.effect { sprite = asset.sprite("small-puff"), anim = 1, palette = 0 }

-- The panel the bomb takes: on the field, solid, with no body on it (0x080E4E26).
local STANDS, BODIES = 0x0001_0010, 0x0F80_0000
-- The other side's solid panels (0x080E4D74, 0x080E4D7C by the user's side: the owner bit 0x20 set for side 1's,
-- clear for side 0's), and those of them with nothing on them (0x080E4D84, 0x080E4D8C).
local SOLID, SIDE_1, OCCUPIED = 0x10, 0x20, 0x0F80_0080

-- The conditions for the other side's panels: solid, and with `forbid`.
local function other_side(side: number, forbid: number): (number, number)
    if side == 0 then
        return bit32.bor(SOLID, SIDE_1), forbid
    end
    return SOLID, bit32.bor(forbid, SIDE_1)
end

-- 0x080E4D94: from the user's back edge (column 1 for side 0, 6 for side 1) forward, the first column with a panel
-- meeting (`need`, `forbid`) in any row; 0 when none.
local function front_column(side: number, need: number, forbid: number): number
    local x, step = 1, 1
    if side == 1 then
        x, step = 6, -1
    end
    while x >= 1 and x <= 6 do
        for y = 1, 3 do
            local f = field.flags(x, y)
            if bit32.band(f, forbid) == 0 and bit32.band(f, need) == need then
                return x
            end
        end
        x += step
    end
    return 0
end

-- In column `x`, rows 3 to 1, the panels meeting (`need`, `forbid`).
local function rows_of(x: number, need: number, forbid: number): { number }
    local found = {}
    for y = 3, 1, -1 do
        local f = field.flags(x, y)
        if bit32.band(f, forbid) == 0 and bit32.band(f, need) == need then
            table.insert(found, y)
        end
    end
    return found
end

-- 0x080E4CC8: the other side's front column, and a random row of it whose panel has nothing on it (any of its solid
-- panels when none has); nil when the other side has no solid panel.
local function target(me: Object): (number?, number?)
    local side = me.alliance
    local need, forbid = other_side(side, 0)
    local x = front_column(side, need, forbid)
    if x == 0 then
        return nil, nil
    end
    local free_need, free_forbid = other_side(side, OCCUPIED)
    local rows = rows_of(x, free_need, free_forbid)
    if #rows == 0 then
        rows = rows_of(x, need, forbid)
        if #rows == 0 then
            return nil, nil
        end
    end
    return x, rows[battle.rng_positive() % #rows + 1]
end

-- 0x080E4DE4: the bomb on the target, if it may stand there, with the damage plus the bonus; else a puff there.
local function place(me: Object)
    local x, y = target(me)
    if not (x and y) then
        -- (0x080E4DF2: with no solid panel on the other side the original reads its panel filter from open bus,
        -- 0x00010010 taken for a table's address, and fills a list through the caller's stale r7.)
        error("TimeBomb's placement found no solid panel of the other side (0x080E4DF2 reads open bus)")
    end
    if field.check(x, y, STANDS, BODIES) then
        local s = me.state :: { bonus: number, variant: CountdownVariant }
        countdown.spawn_kind(KIND, s.variant, x, y, me.alliance, dimming_chips.damage_plus(me, s.bonus), s.bonus)
    else
        local cx, cy = field.center(x, y)
        battle.effect(Vec3.new(cx, cy, 0x10_0000), NO_BOMB)
    end
end

local CONTROLLER = controller.make("timebomb/controller", place)

-- TimeBomb (EXE4 chip 0x061).
local timebomb: Chip = {
    codes = { "J", "K", "L" },
    element = "null",
    family = "summon",
    class = "standard",
    rarity = 2,
    mb = 55,
    damage = 150,
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    dimming = CONTROLLER.hook(BOMB),
}

return { timebomb = timebomb }
