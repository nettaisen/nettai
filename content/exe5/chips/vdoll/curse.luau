--!strict
-- EXE5's VDoll curse (effect object #0x4E, 0x080E6110: EXE6's `sub_80E6088` table, EXE5's own marking and hit): a
-- dimming controller the doll (./doll) starts when it is first hurt after landing, its telop EXE5's Curse (chip 0x174,
-- ../curse/chip). Unlike EXE6's, which hits every combatant of the other side, it picks one panel at random (an RNG2
-- draw) of those the other side's combatants stand on (0x080E625E); with none it keeps the doll's aim (its parameters'
-- third and fourth bytes, which it spawned with), and a column of 0 ends it at once. It marks that panel for 60
-- ticks, waits 10, then hits it with the damage the doll took (0x080E62AC: EXE5's row 0x17, a thrown piercing break,
-- against navis, spark 6, hit modifier 1; EXE6's the curse row, hit modifier 3) and sparkles over whoever stands
-- there; 30 ticks later the dimming ends.

local collision = require("../../rules/collision")
local curse_chip = require("../curse")
local dimming_chips = require("@exelib/dimming")
local panels = require("@exelib/panels")
local regions = require("@exelib/regions")
local sparkles = require("./sparkles")
local sparks = require("../../lib/sparks")

local curse = {}

export type State = {
    -- The panel it marks and hits (Param3, Param4).
    target_x: number,
    target_y: number,
}

-- The doll's aim (its state's).
type DollAim = { target_x: number, target_y: number }

-- The effect's phases.
local MARK, WAIT, HIT = 0x0, 0x4, 0x8
local MARK_TICKS, WAIT_TICKS, HIT_TICKS = 0x3C, 0xA, 0x1E
-- The mark's sound (0x08009620, EXE6's `sub_800AE90`).
local MARK_SOUND = asset.sound("err-select-91")
local HIT_MOD = 1
-- Where the other side's combatants stand (0x080E629C, by the controller's side: require, forbid 0).
local OTHER_SIDE: { [number]: number } = { [0] = 0x0020_0000, [1] = 0x0040_0000 }
-- Who stands on the hit panel (0x080E7356): its table is EXE6's `byte_80E74C4` (0x00200000, 0x00400000), but EXE5's
-- code indexes it by the side unscaled, so side 1 reads the word one byte in (an unaligned load: the first word
-- rotated, 0x00002000).
local OCCUPANT: { [number]: number } = { [0] = 0x0020_0000, [1] = 0x0000_2000 }
-- The sparkles over whoever is hit: 32 ticks of effect 0x24 (EXE5's table, 0x080DFC94: sheet 14-00 in palette 1).
local SPARKLES_LENGTH = 2
local SPARKLE = define.effect { sprite = asset.sprite("explosion"), anim = 0, palette = 1 }

-- 0x080E625E: one of the panels the other side's combatants stand on (bottom right first), at random; none, the aim
-- stays.
local function pick_target(me: Object, s: State)
    local rule = OTHER_SIDE[me.alliance]
    if not rule then
        error(string.format("the curse's side %d reads past its table (0x080E629C)", me.alliance))
    end
    local list = panels.filtered(rule, 0)
    if #list == 0 then
        return
    end
    local p = list[battle.rng_positive() % #list + 1]
    s.target_x, s.target_y = panels.x(p), panels.y(p)
end

-- 0x080E7356: the combatant (one of either side's still in) whose collision stands on (x, y), when the panel's flags
-- have the side's word.
local function occupant(x: number, y: number, side: number): Object?
    local flags = OCCUPANT[side]
    if not flags then
        error(string.format("the curse's side %d reads past 0x080E7394", side))
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

-- 0x080E62AC: the hit on its panel (running while dimmed), and sparkles over whoever stands there.
local function strike(me: Object, s: State)
    local x, y = s.target_x, s.target_y
    local word = int.u32(me.damage + me.stamina * 0x10000)
    local box = battle.hitbox(me, {
        panel_x = x,
        panel_y = y,
        element = 0,
        z = 0,
        region = regions.single,
        hit_effect = sparks.heavy,
        target = collision.hits_navis,
        self_type = collision.thrown_piercing_break,
        damage = bit32.band(word, 0xFFFF),
        stamina = bit32.rshift(word, 16),
        hit_mod = HIT_MOD,
    })
    if box then
        box.run_while_dimmed = true
    end
    local o = occupant(x, y, me.alliance)
    if o then
        sparkles.spawn(o, SPARKLES_LENGTH, SPARKLE, false)
    end
end

-- 0x080E6154: by phase.
local function effect(me: Object)
    local s = me.state :: State
    local phase = me.phase
    if phase == MARK then
        -- 0x080E6174: its panel (a column of 0: straight to the undim), marked for 60 ticks.
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
        battle.warn(MARK_SOUND, Vec3.new(x, y, 0))
        local t = me.timer - 1
        me.timer = bit32.band(t, 0xFFFF)
        if t <= 0 then
            me.phase, me.phase_init = WAIT, 0
        end
    elseif phase == WAIT then
        -- 0x080E61F8.
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
        -- 0x080E6216.
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
        error(string.format("the curse's phase %#x reads past its table (0x080E6168)", phase))
    end
end

curse.kind = define.kind {
    id = "vdoll/curse",
    pool = "effect",
    state = { target_x = "u8", target_y = "u8" },
    -- 0x080E6110.
    update = dimming_chips.phases { name = "the curse", effect = effect },
}

-- What its telop names (the doll spawns it with Curse, bonus 0).
curse.telop = { chip = curse_chip } :: DimmingTelop

-- 0x080E623C: the curse of `doll`, on its panel and side, with the damage word (the damage the doll took, hit
-- parameter 0xB2) and the doll's parameters (its aim). (Its position is the spawner's registers, which nothing reads:
-- the doll's row, 0, 0.)
function curse.spawn(doll: Object, damage: number): Object?
    local o = battle.spawn(curse.kind, Vec3.new(doll.panel_y, 0, 0))
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

return curse
