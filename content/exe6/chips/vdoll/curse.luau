--!strict
-- VDoll's curse (the original's effect object #0x4E, `sub_80E6088`): a
-- dimming controller the doll (doll.luau) starts when it is first hurt
-- after landing, its telop Curse (chip 0x174). It marks every combatant of
-- the other side for 60 ticks, waits 10, then hits each one's panel with
-- the damage the doll took, and sparkles over whoever stands there; 30
-- ticks later the dimming ends. See docs/engine/shot-chips.md §14.2.

local collision = require("../../rules/collision")
local curse_chip = require("../curse").curse
local dimming_chips = require("@exelib/dimming")
local regions = require("@exelib/regions")
local sparkles = require("./sparkles")
local sparks = require("../../lib/sparks")

local curse = {}

export type State = {
    -- The other side's combatants' panels, x | y << 4 (ExtraVars), and
    -- how many (Param3).
    targets: { number },
    count: number,
}

-- The effect's phases.
local MARK, WAIT, HIT = 0x0, 0x4, 0x8
local MARK_TICKS, WAIT_TICKS, HIT_TICKS = 0x3C, 0xA, 0x1E
-- The marks' sound (`sub_800AE90`).
local MARK_SOUND = asset.sound("err-select-91")
-- The hit on each panel (`sub_80E621C`: region 1, spark 6, a curse that
-- hits navis, hit modifier 3, null).
local HIT_MOD = 3
-- The panel flag that says something of the other side's stands there
-- (`byte_80E74C4`, by the controller's side).
local OCCUPIED = { [0] = 0x0020_0000, [1] = 0x0040_0000 }
-- The sparkles over whoever is hit: 32 ticks of effect 0x24.
-- (Its length, `dword_80E19F0`'s row 2, the curse's first parameter.)
local SPARKLES_TICKS = 0x20
local SPARKLE = new.effect { sprite = asset.sprite("explosion"), anim = 0, palette = 1 }

-- `sub_80E7486`: the combatant (one of either side's still in) whose
-- collision stands on (x, y), when the panel says one of `side`'s
-- opponents is there.
local function occupant(x: number, y: number, side: number): Object?
    if not field.check(x, y, OCCUPIED[side], 0) then
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

-- `sub_80E621C`: the hit on a target's panel (running while dimmed), and
-- sparkles over whoever stands there.
local function strike(me: Object, packed: number)
    local x, y = bit32.band(packed, 7), bit32.rshift(packed, 4)
    local word = int.u32(me.damage + me.stamina * 0x10000)
    local box = battle.hitbox(me, {
        panel_x = x,
        panel_y = y,
        element = 0,
        z = 0,
        region = regions.single,
        hit_effect = sparks.heavy,
        target = collision.hits_navis,
        self_type = collision.curse,
        damage = bit32.band(word, 0xFFFF),
        stamina = bit32.rshift(word, 16),
        hit_mod = HIT_MOD,
    })
    if box then
        box.run_while_dimmed = true
    end
    local o = occupant(x, y, me.alliance)
    if o then
        sparkles.spawn(o, SPARKLES_TICKS, SPARKLE, false)
    end
end

-- `sub_80E60CC`: by phase.
local function effect(me: Object)
    local s = me.state :: State
    local phase = me.phase
    if phase == MARK then
        -- `sub_80E60EC`: the other side's combatants; none, straight to
        -- the undim. Their panels marked for 60 ticks.
        if me.phase_init == 0 then
            local n = 0
            for _, o in battle.alive_actors(bit32.bxor(me.alliance, 1)) do
                n += 1
                s.targets[n] = bit32.band(o.panel_x + o.panel_y * 0x10, 0xFF)
            end
            s.count = n
            if n == 0 then
                dimming_chips.done(me)
                return
            end
            me.timer = MARK_TICKS
            me.phase_init = 4
            return
        end
        for i = 1, s.count do
            local at = s.targets[i]
            local x, y = field.center(bit32.band(at, 7), bit32.rshift(at, 4))
            battle.warn(MARK_SOUND, Vec3.new(x, y, 0))
        end
        local t = me.timer - 1
        me.timer = bit32.band(t, 0xFFFF)
        if t <= 0 then
            me.phase, me.phase_init = WAIT, 0
        end
    elseif phase == WAIT then
        -- `sub_80E6180`.
        if me.phase_init == 0 then
            me.timer = WAIT_TICKS
            me.phase_init = 4
        end
        local t = me.timer - 1
        me.timer = bit32.band(t, 0xFFFF)
        if t <= 0 then
            me.phase, me.phase_init = HIT, 0
        end
    elseif phase == HIT then
        -- `sub_80E619E`.
        if me.phase_init == 0 then
            for i = 1, s.count do
                strike(me, s.targets[i])
            end
            me.timer = HIT_TICKS
            me.phase_init = 4
        end
        local t = me.timer - 1
        me.timer = bit32.band(t, 0xFFFF)
        if t <= 0 then
            dimming_chips.done(me)
        end
    else
        error(string.format("the curse's phase %#x reads past off_80E60E0", phase))
    end
end

curse.kind = new.kind {
    id = "vdoll/curse",
    pool = "effect",
    state = { targets = "u8[4]", count = "u8" },
    -- `sub_80E6088`.
    update = dimming_chips.phases { name = "the curse", effect = effect },
}

-- What its telop names (the doll spawns it with Curse, bonus 0).
curse.telop = { chip = curse_chip } :: DimmingTelop

-- `sub_80E61D2`: the curse of `doll`, on its panel and side, with the
-- damage word (the damage the doll took, hit parameter 0xB2). (Its
-- position is the spawner's registers, which nothing reads; its telop's
-- chip is `curse.telop`.)
function curse.spawn(doll: Object, damage: number): Object?
    local o = battle.spawn(curse.kind, Vec3.new(doll.panel_y, 0, 0))
    if o then
        o.panel_x, o.panel_y = doll.panel_x, doll.panel_y
        o.element = 0
        o.related1 = doll
        o.alliance = doll.alliance
        o:set_damage_word(damage)
    end
    return o
end

return curse
