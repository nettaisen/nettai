--!strict
-- BN5's TimeBom chips' bomb and controller (TimeBom1-3 and TimeBom+, the dimming handler's subtype 10): the shared
-- code (@exelib/timebom: BN6's, BN5's attack object #0x4B and effect object #0x27) but for
--
-- - where the bomb goes (0x080E3420, BN6's `sub_80E3264`): in each row, the enemy's frontmost panel (from the
--   back of the field, the first panel of the other side's, then the first of the user's, one step back toward
--   the enemy), if it is solid with no body on it; a random one of those rows (RNG2);
-- - how the bomb leaves (0x080CF21C): any removal leaves a puff (BN5 neither absorbs nor blinks obstacles out);
-- - TimeBom+'s bomb's HP: 300 (BN6's 200, 0x080CEEE4's row 1);
-- - the bombs' identities (actor records 0xD8 and 0xD9): field objects of AI index 0x21 (BN6's 1), which nothing
--   of BN5's reads but the record (their hooks are none, 0x0800ED30 and 0x0800EEE0);
-- - BN5's look and collision types, and its action table of 8 rows (no frozen or bubbled: the countdown and the
--   blink are its 6 and 7, BN6's 8 and 9, the same actions).

local collision = require("../../rules/collision")
local controller = require("@exelib/timebom/controller")
local countdown = require("@exelib/timebom/countdown")
local dimming_chips = require("@exelib/dimming")
local effects = require("../../lib/effects")
local panel_bursts = require("../../objects/panel-bursts")

local timebom = {}

local SPRITE = asset.sprite("countdown-bomb")

timebom.kind = countdown.make_kind("timebom/countdown", {
    sprite = SPRITE,
    sounds = {
        place = asset.sound("appear"),
        tick = asset.sound("tick"),
        last = asset.sound("last"),
        blast = asset.sound("burst"),
        puff = asset.sound("hit-bomb-1"),
    },
    blast = { is = collision.attack, hits = collision.hits_navis },
    removal_puffs = true,
    -- (The blast's explosions and the puff BN6's.)
    explosion = effects.explosion,
    puff = effects.puff,
    bursts = panel_bursts.kind,
})

-- What the bombs are and what they react to: a neutral body, hit by everything (rows 0xE and 0xF).
local BODY = { is = collision.neutral_body, hits = collision.everything }

-- The bombs (0x080CEEE4's rows 0 and 1).
timebom.variants = {
    bomb = countdown.variant {
        appear_anim = 0,
        idle_anim = 1,
        hp = 50,
        identity = define.identity {
            id = "timebomb",
            class = "field_object",
            version = 5,
            ai_index = 0x21,
            look = { sprite = SPRITE, anim = 1, palette = 0, shadow = true, keeps_flip = true },
        },
        class = 1,
        collision = BODY,
    },
    big = countdown.variant {
        appear_anim = 2,
        idle_anim = 3,
        hp = 300,
        identity = define.identity {
            id = "timebomb-big",
            class = "field_object",
            version = 0,
            ai_index = 0x21,
            look = { sprite = SPRITE, anim = 3, palette = 0, shadow = true, keeps_flip = true },
        },
        class = 1,
        collision = BODY,
        strong_shake = true,
    },
}

-- Where the search for each row's panel starts and what it looks for, by side (0x080E351C: the other side's
-- panels; 0x080E34B0: the user's), and what the panel the bomb goes on must be.
local OTHER_SIDES: { [number]: PanelCondition } = {
    [0] = { require = 0x20, forbid = 0 },
    [1] = { require = 0, forbid = 0x20 },
}
local OWN_SIDES: { [number]: PanelCondition } = {
    [0] = { require = 0, forbid = 0x20 },
    [1] = { require = 0x20, forbid = 0 },
}
local SOLID, BODIES = 0x10, 0x0F80_0000

-- `object_getFirstPanelInDirectionFiltered`: from (x, y) by `step` columns, the first panel meeting `cond`, while
-- the panels are on the field (nil: none).
local function first(x: number, y: number, cond: PanelCondition, step: number): number?
    while true do
        if field.check(x, y, cond.require, cond.forbid) then
            return x
        end
        x += step
        if not field.valid(x, y) then
            return nil
        end
    end
end

-- 0x080E3420: the panel the bomb goes on (nil: none).
local function target(me: Object): (number?, number?)
    local flip = bit32.bxor(me.alliance, me.flip)
    local start = bit32.bxor(flip, 1) * 5 + 1
    local back = -me.facing
    local rows: { { number } } = {}
    for y = 1, 3 do
        local far = first(start, y, OTHER_SIDES[me.alliance], back)
        -- (The game tests the column for zero: column 0 is off the field.)
        if far and far ~= 0 then
            local near = first(far, y, OWN_SIDES[me.alliance], back)
            if near and near ~= 0 then
                local x = near + me.facing
                if field.check(x, y, SOLID, BODIES) then
                    table.insert(rows, { x, y })
                end
            end
        end
    end
    if #rows == 0 then
        return nil, nil
    end
    local pick = rows[battle.rng_positive() % #rows + 1]
    return pick[1], pick[2]
end

-- 0x080E34C0: the bomb, with the damage plus the bonus.
local function place(me: Object)
    local x, y = target(me)
    if x and y then
        local s = me.state :: { bonus: number, variant: CountdownVariant }
        countdown.spawn_kind(timebom.kind, s.variant, x, y, me.alliance, dimming_chips.damage_plus(me, s.bonus), s.bonus)
    end
end

timebom.controller = controller.make("timebom/controller", place)

return timebom
