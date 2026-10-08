--!strict
-- VDoll's curse, as EXE5 and EXE4 share it (effect object #0x4E: EXE5's 0x080E6110, EXE4's 0x080E77E8, the same code;
-- EXE6's `sub_80E6088` table, but EXE6's marking and hit are its own), made for a game (`curse.make`): a dimming
-- controller the doll (@exelib/vdoll/doll) starts when it is first hurt after landing, its telop the game's Curse
-- (chip 0x174). Unlike EXE6's, which hits every combatant of the other side, it picks one panel at random (an RNG2
-- draw) of those the other side's combatants stand on (EXE5's 0x080E625E, EXE4's 0x080E7936); with none it keeps the
-- doll's aim (its parameters' third and fourth bytes, which it spawned with), and a column of 0 ends it at once. It
-- marks that panel for 60 ticks (the game's warning sound), waits 10, then hits it with the damage the doll took (a
-- thrown piercing break against navis, the heavy spark, hit modifier 1: the region words EXE5's 0x080E62DC, EXE4's
-- 0x080E79B4) and sparkles over whoever stands there (the game's sparkles: 32 ticks of its effect); 30 ticks later the
-- dimming ends.

local dimming_chips = require("@exelib/dimming")
local panels = require("@exelib/panels")
local regions = require("@exelib/regions")

local curse = {}

export type State = {
    -- The panel it marks and hits (Param3, Param4).
    target_x: number,
    target_y: number,
}

-- The doll's aim (its state's).
type DollAim = { target_x: number, target_y: number }

-- A game's curse: its kind's id; the Curse chip its telop names; the warning's sound as it marks; its hit's spark and
-- collision types; the game's sparkles (`spawn` of @exelib/vdoll/sparkles) and the effect they show.
export type Look = {
    id: string,
    chip: Chip,
    mark_sound: Sound,
    spark: Spark,
    is: CollisionType,
    hits: CollisionType,
    sparkles: { spawn: (over: Object, ticks: number, effect: Effect, silent: boolean) -> Object? },
    sparkle: Effect,
}

-- The curse made for a game.
export type Curse = { kind: Kind, telop: DimmingTelop, spawn: (doll: Object, damage: number) -> Object? }

-- The effect's phases.
local MARK, WAIT, HIT = 0x0, 0x4, 0x8
local MARK_TICKS, WAIT_TICKS, HIT_TICKS = 0x3C, 0xA, 0x1E
local HIT_MOD = 1
-- Where the other side's combatants stand (EXE5's 0x080E629C, EXE4's 0x080E7974, by the controller's side: require,
-- forbid 0).
local OTHER_SIDE: { [number]: number } = { [0] = 0x0020_0000, [1] = 0x0040_0000 }
-- Who stands on the hit panel (EXE5's 0x080E7356, EXE4's 0x080E89BA): its table is EXE6's `byte_80E74C4`
-- (0x00200000, 0x00400000), but the code indexes it by the side unscaled, so side 1 reads the word one byte in (an
-- unaligned load: the first word rotated, 0x00002000).
local OCCUPANT: { [number]: number } = { [0] = 0x0020_0000, [1] = 0x0000_2000 }
-- The sparkles' length (`dword_80E19F0`'s row 2, the curse's first parameter to them).
local SPARKLES_TICKS = 0x20

-- One of the panels the other side's combatants stand on (bottom right first), at random; none, the aim stays.
local function pick_target(me: Object, s: State)
    local rule = OTHER_SIDE[me.alliance]
    if not rule then
        error(string.format("the curse's side %d reads past its table", me.alliance))
    end
    local list = panels.filtered(rule, 0)
    if #list == 0 then
        return
    end
    local p = list[battle.rng_positive() % #list + 1]
    s.target_x, s.target_y = panels.x(p), panels.y(p)
end

-- The combatant (one of either side's still in) whose collision stands on (x, y), when the panel's flags have the
-- side's word.
local function occupant(x: number, y: number, side: number): Object?
    local flags = OCCUPANT[side]
    if not flags then
        error(string.format("the curse's side %d reads past its occupant table", side))
    end
    if not field.check(x, y, flags, 0) then
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

function curse.make(look: Look): Curse
    -- EXE5's 0x080E62AC, EXE4's 0x080E7984: the hit on its panel (running while dimmed), and sparkles over whoever
    -- stands there.
    local function strike(me: Object, s: State)
        local x, y = s.target_x, s.target_y
        local word = int.u32(me.damage + me.stamina * 0x10000)
        local box = battle.hitbox(me, {
            panel_x = x,
            panel_y = y,
            element = 0,
            z = 0,
            region = regions.single,
            hit_effect = look.spark,
            target = look.hits,
            self_type = look.is,
            damage = bit32.band(word, 0xFFFF),
            stamina = bit32.rshift(word, 16),
            hit_mod = HIT_MOD,
        })
        if box then
            box.run_while_dimmed = true
        end
        local o = occupant(x, y, me.alliance)
        if o then
            look.sparkles.spawn(o, SPARKLES_TICKS, look.sparkle, false)
        end
    end

    -- EXE5's 0x080E6154, EXE4's 0x080E7834: by phase.
    local function effect(me: Object)
        local s = me.state :: State
        local phase = me.phase
        if phase == MARK then
            -- Its panel (a column of 0: straight to the undim), marked for 60 ticks.
            if me.phase_init == 0 then
                pick_target(me, s)
                if s.target_x == 0 then
                    dimming_chips.done(me)
                    return
                end
                me.timer = MARK_TICKS
                me.phase_init = 4
                return
            end
            local x, y = field.center(s.target_x, s.target_y)
            battle.warn(look.mark_sound, Vec3.new(x, y, 0))
            local t = me.timer - 1
            me.timer = bit32.band(t, 0xFFFF)
            if t <= 0 then
                me.phase, me.phase_init = WAIT, 0
            end
        elseif phase == WAIT then
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
            if me.phase_init == 0 then
                strike(me, s)
                me.timer = HIT_TICKS
                me.phase_init = 4
            end
            local t = me.timer - 1
            me.timer = bit32.band(t, 0xFFFF)
            if t <= 0 then
                dimming_chips.done(me)
            end
        else
            error(string.format("the curse's phase %#x reads past its table", phase))
        end
    end

    local kind = new.kind {
        id = look.id,
        pool = "effect",
        state = { target_x = "u8", target_y = "u8" },
        -- EXE5's 0x080E6110, EXE4's 0x080E77E8.
        update = dimming_chips.phases { name = "the curse", effect = effect },
    }

    -- EXE5's 0x080E623C, EXE4's 0x080E7914: the curse of `doll`, on its panel and side, with the damage word (the
    -- damage the doll took, hit parameter 0xB2) and the doll's parameters (its aim). (Its position is the spawner's
    -- registers, which nothing reads: the doll's row, 0, 0.)
    local function spawn(doll: Object, damage: number): Object?
        local o = battle.spawn(kind, Vec3.new(doll.panel_y, 0, 0))
        if o then
            local aim = doll.state :: DollAim
            local s = o.state :: State
            s.target_x, s.target_y = aim.target_x, aim.target_y
            o.panel_x, o.panel_y = doll.panel_x, doll.panel_y
            o.element = 0
            o.related1 = doll
            o.alliance = doll.alliance
            o:set_damage_word(damage)
        end
        return o
    end

    -- What its telop names (the doll spawns it with Curse, bonus 0).
    return { kind = kind, telop = { chip = look.chip } :: DimmingTelop, spawn = spawn }
end

return curse
