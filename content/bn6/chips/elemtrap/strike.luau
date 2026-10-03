--!strict
-- ElemTrap's counterattack (the original's effect object #0x2B,
-- `sub_80E35A4`): a dimming controller its trap (./trap) starts when it
-- springs, right after the trap in the update list (the Japanese games'
-- placement; see `strike.spawn`). The usual dimming phases
-- (lib/dimming; its telop names the trap's chip, ElemTrap, with its bonus),
-- with an effect that strikes every panel the enemy stands on in the
-- element that sprang the trap (the damage plus the bonus; elec and wood
-- hits carry a status too), sets off bursts over the whole field
-- (objects/panel-bursts), and waits 61 ticks. A game whose strikes hit
-- otherwise makes its kind with `strike.make_kind(id, look)` (BN5's).

local collision = require("../../rules/collision")
local dimming_chips = require("@common/dimming")
local effects = require("../../lib/effects")
local panel_bursts = require("../../objects/panel-bursts/panel_bursts")
local regions = require("@common/regions")
local sparks = require("../../lib/sparks")
local statuses = require("../../rules/status")

export type State = {
    -- The Atk+ / cross bonus (object +0x32), which adds to the damage.
    bonus: number,
}

local strike = {}

local WAIT = 0x3C

-- `byte_80E36E4`: the panels struck, by side: any with the enemy's navi,
-- other bodies or neutral objects (flags 0x01A00000 for side 0,
-- 0x02C00000 for side 1).
local STRUCK: { [number]: number } = { [0] = 0x01A0_0000, [1] = 0x02C0_0000 }

-- The bursts' looks: water for aqua, lightning for elec, leaves for wood
-- (the original's panel-burst rows 0, 2 and 4; fire and the null element
-- take the shells' fiery burst, row 3).
local SPLASH = define.effect { sprite = asset.sprite("bn6:spout-splash"), anim = 0, palette = 0 }
local LIGHTNING = define.effect { sprite = asset.sprite("bn6:lightning"), anim = 0, palette = 0 }
local LEAVES = define.effect { sprite = asset.sprite("bn6:hit-sparks"), anim = 5, palette = 0 }

-- The strike by element (null, fire, aqua, elec, wood): its sound
-- (`byte_80E3714`; none for the null element), its hit (`byte_80E36EC`:
-- the hit spark, hit modifier and status; every one a single panel's, an
-- attack that reaches navis) and the bursts' look (`byte_80E371E`).
local STRIKES: { [number]: ElemTrapStrike } = {
    [0] = { sound = nil, spark = sparks.breaking, hit_mod = 3, status = nil, burst = effects.shell_burst },
    [1] = {
        sound = asset.sound("bn6:target-move"),
        spark = sparks.fire,
        hit_mod = 3,
        status = nil,
        burst = effects.shell_burst,
    },
    [2] = { sound = asset.sound("bn6:spout-ball"), spark = sparks.aqua, hit_mod = 3, status = nil, burst = SPLASH },
    [3] = {
        sound = asset.sound("bn6:beast-over-burst"),
        spark = sparks.elec,
        hit_mod = 1,
        status = statuses.paralyze_90,
        burst = LIGHTNING,
    },
    [4] = { sound = asset.sound("bn6:junk-shot"), spark = sparks.wood, hit_mod = 1, status = statuses.confuse_480, burst = LEAVES },
}
-- The bursts go over the whole field, one every 2 ticks.
local BURST_INTERVAL = 2

-- BN6's look: the strikes by element, and what the hits are and reach.
local BN6: ElemTrapStrikeLook = { strikes = STRIKES, self_type = collision.attack, target = collision.hits_navis }

-- `sub_80E362C`: strike the panels, from (6, 3) back to (1, 1), then the
-- bursts. (Nothing to strike: nothing happens.)
local function hit_panels(me: Object, look: ElemTrapStrikeLook)
    local mask = STRUCK[me.alliance]
    local list = {}
    for x = 1, 6 do
        for y = 1, 3 do
            if bit32.btest(field.flags(x, y), mask) then
                table.insert(list, { x = x, y = y })
            end
        end
    end
    if #list == 0 then
        return
    end
    local e = me.element
    local by_element = look.strikes[e]
    if by_element == nil then
        error(string.format("ElemTrap's counterattack: element %d reads past its tables", e))
    end
    local sound = by_element.sound
    if sound then
        battle.play_sound(sound)
    end
    local s = me.state :: State
    local word = int.u32(bit32.bor(me.damage, bit32.lshift(me.stamina, 16)) + s.bonus)
    for i = #list, 1, -1 do
        local p = list[i]
        local box = battle.hitbox(me, {
            panel_x = p.x,
            panel_y = p.y,
            element = e,
            z = 0,
            region = regions.single,
            hit_effect = by_element.spark,
            target = look.target,
            self_type = look.self_type,
            damage = bit32.band(word, 0xFFFF),
            stamina = bit32.rshift(word, 16),
            hit_mod = by_element.hit_mod,
            status = by_element.status,
        })
        if box then
            box.run_while_dimmed = true
        end
    end
    panel_bursts.spawn(me, regions.field, BURST_INTERVAL, 0, by_element.burst)
end

-- A counterattack kind (`id`) of `look`.
function strike.make_kind(id: string, look: ElemTrapStrikeLook): Kind
    -- `sub_80E35E8`: the strike, then 61 ticks.
    local function effect(me: Object)
        if me.phase_init == 0 then
            me.timer = WAIT
            hit_panels(me, look)
            me.phase_init = 4
        end
        local t = me.timer - 1
        me.timer = t
        if t < 0 then
            dimming_chips.done(me)
        end
    end

    return define.kind {
        id = id,
        pool = "effect",
        state = { bonus = "u16" },
        -- `sub_80E35A4`.
        update = dimming_chips.phases { name = "ElemTrap's counterattack", effect = effect },
    }
end

strike.kind = strike.make_kind("bn6:elemtrap/strike", BN6)

-- `sub_80E360E`: the counterattack for `user`, of `element`, with the
-- trap's damage word and bonus. It goes right after the trap in the update
-- list, so it first runs later this tick: the Japanese games' routine (EXE6
-- Falzar 0x080E81E6, EXE6 Gregar 0x080E9516) spawns it with
-- `object_spawnType4`, as `battle.spawn` does. The US games' spawns it at
-- the head of the list (`sub_80033E4`, US only), so it first runs next
-- tick; the content follows the Japanese games on every console (the
-- user's decision, 2026-10-02: docs/engine/jp-differences.md §8.1). (Its
-- position is the spawner's registers, which nothing reads.) Of `kind`,
-- BN6's by default.
function strike.spawn(user: Object, element: number, damage: number, bonus: number, kind: Kind?): Object?
    local o = battle.spawn(kind or strike.kind, Vec3.new(0, element, 0))
    if not o then
        return nil
    end
    o.element = element
    o.related1 = user
    o.alliance = user.alliance
    o:set_damage_word(damage)
    local s = o.state :: State
    s.bonus = bonus
    return o
end

return strike
