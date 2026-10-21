--!strict
-- ShadowMan's ambush (EXE5 DS's party battle: 0x020040C4, from the navi's low-HP check 0x02098A84 when Mag-Barr doesn't
-- come; his actor the support actor's variant 4, 08-08, 0x021FEC74; his strikes the DS's effect object 0x96, 0x02226504;
-- docs/design/exe5ds-map.md §4.7): once a battle, the navi idle at a quarter of its HP or less (and below its maximum),
-- with the light test passed and ShadowMan a benched member who may support, he appears on a panel of the field drawn
-- with RNG2, readies, and strikes nine times, 8 ticks apart, alternately at an enemy and at an empty enemy panel: each
-- strike marks its panel for 15 ticks, then bursts there (no damage) with one status for all nine, drawn as he readies.

local collision = require("../../rules/collision")
local effects = require("../../lib/effects")
local panels = require("@exelib/panels")
local party = require("../../rules/party")
local party_support = require("../../lib/party_support")
local regions = require("@exelib/regions")
local shadowman = require("../shadowman").shadowman
local status = require("../../rules/status")

local support = {}

-- Where he may appear (0x020CC3A8), by side: of the side's own area, nothing on it or reserved.
local APPEAR_REQUIRE = { [0] = 0, [1] = 0x20 }
local APPEAR_FORBID = { [0] = 0x0F8800A0, [1] = 0x0F880080 }

-- His readying (animation 0x11, 31 ticks), his strikes and the ticks between them.
local ANIM_READY, READY_TICKS, STRIKES, STRIKE_EVERY = 0x11, 0x1F, 9, 8
local READY, STRIKE = 0x0, 0x4

-- The strikes' kinds (0x0222B41C, 0x0222B420): the status each burst carries and the burst's palette.
type Strike = { status: Status, palette: number }
local CONFUSE: Strike = { status = status.confuse_960, palette = 4 }
local IMMOBILIZE: Strike = { status = status.immobilize_120, palette = 1 }
local BLIND: Strike = { status = status.blind_1200, palette = 2 }
local PARALYZE: Strike = { status = status.paralyze_150, palette = 3 }
-- One for the nine, by an RNG2 draw's bits 3–6 (0x02227184).
local STRIKE_KINDS: { Strike } = {
    CONFUSE, CONFUSE, CONFUSE, CONFUSE, CONFUSE, CONFUSE,
    IMMOBILIZE, IMMOBILIZE,
    BLIND, BLIND,
    PARALYZE, PARALYZE, PARALYZE, PARALYZE, PARALYZE, PARALYZE,
}

-- Where a strike lands (0x022271E0, 0x022271F0), by side: the enemy area's panels without an enemy body, or with one.
local OPEN_REQUIRE = { [0] = 0x20, [1] = 0 }
local OPEN_FORBID = { [0] = 0x04000000, [1] = 0x08000020 }
local ENEMY_REQUIRE = { [0] = 0x04000020, [1] = 0x08000000 }
local ENEMY_FORBID = { [0] = 0, [1] = 0x20 }
-- The panel flag of the other side's navi, by side (`sub_80E7486`'s).
local ENEMY_NAVI = { [0] = 0x00200000, [1] = 0x00400000 }

-- A strike: its panel marked every other four ticks, its burst after 15 (the effect table's row 0 in the kind's palette,
-- 0x90's sound) with a hit on the panel: no damage, the kind's status.
local MARKED_TICKS = 0xE
local BURST_SOUND = asset.sound("panel-poison")

type ActorState = { member: number, kind: number, on_enemy: boolean }
type StrikeState = { kind: number }

-- 0x02226524: the mark, then the burst.
local function strike_update(me: Object)
    if me.phase_init == 0 then
        me.timer = 0
        me.phase_init = 4
    end
    if bit32.band(me.timer, 4) == 0 then
        field.highlight(me.panel_x, me.panel_y)
    end
    me.timer += 1
    if me.timer <= MARKED_TICKS then
        return
    end
    local kind = STRIKE_KINDS[(me.state :: StrikeState).kind + 1]
    battle.effect(me.pos, effects.explosion, 0, kind.palette)
    battle.hitbox(me, {
        panel_x = me.panel_x,
        panel_y = me.panel_y,
        element = me.element,
        z = me.pos.z,
        region = regions.single,
        target = collision.hits_navis,
        self_type = collision.attack,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = 0,
        status = kind.status,
    })
    battle.play_sound(BURST_SOUND)
    me:set_lifecycle("destroy")
end

support.strike = new.kind {
    id = "shadowman/ambush-strike",
    pool = "effect",
    state = { kind = "u8" },
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            -- 0x02226514: on its panel.
            me:set_coordinates_from_panel()
            me:set_lifecycle("update")
            strike_update(me)
        elseif l == "update" then
            strike_update(me)
        else
            me:free()
        end
    end,
}

-- 0x02226594: a strike of the kind (by its index in the draw's table) on (x, y), ShadowMan's side's, with his damage word.
local function send_strike(me: Object, x: number, y: number, kind: number)
    local o = battle.spawn(support.strike)
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = 0
        o.related1 = me
        o.alliance, o.flip = me.alliance, me.flip
        o:set_damage_word(bit32.bor(me.damage, bit32.lshift(me.stamina, 16)));
        (o.state :: StrikeState).kind = kind
    end
end

-- `sub_80E7486`: if the panel shows the other side's navi (of `side`), the first combatant (both sides' lists) whose
-- collision is on it.
local function combatant_on(x: number, y: number, side: number): Object?
    if not bit32.btest(field.flags(x, y), ENEMY_NAVI[side]) then
        return nil
    end
    for s = 0, 1 do
        for _, o in battle.alive_actors(s) do
            local c = o.collision
            if c.panel_x == x and c.panel_y == y then
                return o
            end
        end
    end
    return nil
end

-- 0x021FF38C: the panel holds a navi of ShadowMan's own side (`sub_80E7486` asked for the other side's) under none of
-- paralysis, confusion, blindness or immobilization; such a panel isn't struck.
local function status_free(x: number, y: number, side: number): boolean
    local o = combatant_on(x, y, 1 - side)
    if o == nil then
        return false
    end
    return o:status_timer("paralyze") == 0
        and o:status_timer("confuse") == 0
        and o:status_timer("immobilize") == 0
        and o:status_timer("blind") == 0
end

-- 0x021FF2C0: of the panels from the enemy area's far edge back across the field, column by column (rows 1 to 3), those
-- with an enemy body (`on_enemy`) or without one, one drawn with RNG2.
local function aim(me: Object, on_enemy: boolean): (number?, number?)
    local side = me.alliance
    local turned = bit32.bxor(side, me.flip)
    local back = if turned == 0 then -1 else 1
    local require = if on_enemy then ENEMY_REQUIRE[side] else OPEN_REQUIRE[side]
    local forbid = if on_enemy then ENEMY_FORBID[side] else OPEN_FORBID[side]
    local found: { number } = {}
    local x = bit32.bxor(turned, 1) * 5 + 1
    repeat
        for y = 1, 3 do
            if field.check(x, y, require, forbid) and not (on_enemy and status_free(x, y, side)) then
                local p: number = panels.pack(x, y)
                table.insert(found, p)
            end
        end
        x += back
    until x < 1 or x > 6
    if #found == 0 then
        return nil, nil
    end
    local p = found[battle.rng_positive() % #found + 1]
    return panels.x(p), panels.y(p)
end

-- 0x021FECF4: he readies (31 ticks; then the strikes' kind is drawn), then strikes: every 8 ticks the next of the two
-- kinds of panel (the other when it has none; neither, no strike, and he tries again on his next tick), nine times.
local function ambush(me: Object)
    local s = me.state :: ActorState
    if me.phase == READY then
        if me.phase_init == 0 then
            me.anim = ANIM_READY
            me.timer = READY_TICKS
            me.phase_init = 4
        end
        local t = me.timer - 1
        me.timer = t
        if t > 0 then
            return
        end
        me.timer2 = STRIKES
        s.kind = bit32.band(bit32.rshift(battle.rng_positive(), 3), 0xF)
        me.phase, me.phase_init = STRIKE, 0
    elseif me.phase == STRIKE then
        if me.phase_init == 0 then
            s.on_enemy = not s.on_enemy
            local x, y = aim(me, s.on_enemy)
            if x == nil then
                s.on_enemy = not s.on_enemy
                x, y = aim(me, s.on_enemy)
            end
            if x and y then
                me.future_panel_x, me.future_panel_y = x, y
                send_strike(me, x, y, s.kind)
                me.timer = STRIKE_EVERY
                me.phase_init = 4
            end
        end
        local t = me.timer - 1
        me.timer = t
        if t > 0 then
            return
        end
        local n = me.timer2 - 1
        me.timer2 = n
        if n > 0 then
            me.phase_init = 0
        else
            party_support.go(me, party_support.WARP)
        end
    else
        error(string.format("ShadowMan's ambush: phase %#x reads past its table (0x0222717C)", me.phase))
    end
end

support.actor = party_support.make {
    id = "shadowman/support",
    navi = shadowman,
    -- (0x021FEC85: no panel aimed at.)
    started = function(me: Object)
        me.future_panel_x, me.future_panel_y = 0xFF, 0xFF
    end,
    appear_ticks = 0x14,
    -- (0x021FEDA9: his parts stay on until he is destroyed.)
    warp_keeps_parts = true,
    actions = { [party_support.ATTACK] = ambush },
    state = { kind = "u8", on_enemy = "bool" },
}

-- 0x020040C4 and 0x02004D84 (the rules' `low_hp`, after Mag-Barr): once a battle (the party block's +0xF, spent even
-- if no panel is free), the light test, ShadowMan a benched member who may support, the navi below its maximum HP; he
-- comes on a panel of the side's area drawn with RNG2 (but the navi's), with no damage, facing as the navi does. True
-- when he came.
function support.low_hp(side: number, navi: Object): boolean
    if not party.light(side) then
        return false
    end
    local m = party.member_of(side, shadowman)
    if m == nil or not party.may_support(side, shadowman) or not party.benched(side, shadowman) then
        return false
    end
    if navi.hp >= navi.max_hp then
        return false
    end
    local s = party.supports(side)
    if s.shadowman_used then
        return false
    end
    s.shadowman_used = true
    if battle.navi(side).navi == shadowman then
        return false
    end
    local found = panels.filtered_except(navi, APPEAR_REQUIRE[side], APPEAR_FORBID[side])
    if #found == 0 then
        return false
    end
    local p = found[battle.rng_positive() % #found + 1]
    return party_support.send(support.actor, navi, panels.x(p), panels.y(p), navi.flip, 0, m) ~= nil
end

return support
