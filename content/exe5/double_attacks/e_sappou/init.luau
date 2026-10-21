--!strict
-- E-SAPPOU (the DS's chip 0x168; Engetsu Sappou), the Double Attack of Colonel and ShadowMan (docs/design/exe5ds-map.md
-- §5, §5.2: its director the DS's effect object 0x9F, its actors its T1 0x40, its rocks its T1 0x44). ShadowMan vanishes
-- in a puff of dust; Colonel raises his arm and drops a rock (./rock) on the panel in front of each enemy and on the one
-- behind it, where the panel is the other side's, solid and empty; each rock lands, waits its turn (30 ticks, 16 more
-- for each rock before it) and becomes a copy of ShadowMan whose moon blade (MoonBld's: a slash of the damage word with
-- hit modifier 3 and bug 0x18, argument 1, an HP drain of level 1) sweeps the eight panels around it. The enemy beside
-- a copy is hit once by it: two hits an enemy, the drain going on after. Once the rocks are gone ShadowMan comes back.
-- The chip's record and name are the DS's (written by tools/exe5/gen_content.py, bn6battle-verify, from the DS ROMs); its
-- record's use is the cannon's (action 0x14, subtype 0: the DS copied Cannon's record), which nothing starts: no folder
-- or hand holds it.
--
-- Its director links the two actors (each its partner's, +0x60), which wait on each other.

local cannon_chips = require("../chips/cannon")
local colonel = require("../navis/colonel").colonel
local double_attack = require("../lib/double_attack")
local effects = require("../lib/effects")
local rock = require("@self/rock")
local shadowman = require("../navis/shadowman").shadowman

-- Colonel's rocks (Param4, +0x07: how many he dropped) and their flags (+0x78 to +0x7F, a byte a rock).
type State = DoubleAttackActor & { partner: Object?, rocks: number, out: { boolean } }

local CHARGE_TICKS = 0x3C
-- Colonel's drop: his animation 0x14, 60 ticks.
local ANIM_DROP, DROP_TICKS = 0x14, 0x3C
local ANIM_IDLE = 0
-- The rocks: the panels the enemies' bodies stand on (flag 0x04000000 for side 0, 0x08000000 for side 1), a rock beside
-- each on a solid empty panel of the other side's (flag 0x10, none of 0x0F800000); each waits 30 ticks and 16 more for
-- each rock before it; eight flags at most.
local ENEMY_BODY = { [0] = 0x04000000, [1] = 0x08000000 }
local SOLID, OCCUPIED = 0x10, 0x0F800000
local FIRST_WAIT, NEXT_WAIT, MOST_ROCKS = 0x1E, 0x10, 8
-- ShadowMan: 4 ticks of animation 4, then gone in a puff of dust 30 pixels up; back (animation 3) for 20 ticks with the
-- appearing sound.
local ANIM_VANISH, VANISH_TICKS, DUST_Z = 4, 4, 0x1E0000
local ANIM_BACK, BACK_TICKS = 3, 0x14
local SOUND_APPEAR = asset.sound("appear")
local AFTER_TICKS = 0x1E

-- The director's link (0x02032EA8): each actor's partner, the other actor it brought.
local function link(me: Object)
    local s = me.state :: State
    for _, o in battle.objects_of(me.kind) do
        local t = o.state :: State
        if o ~= me and t.director == s.director and t.role ~= s.role then
            s.partner = o
            return
        end
    end
end

local function partner(me: Object): Object
    local p = (me.state :: State).partner
    if not p then
        error("E-SAPPOU's actor has no partner (the DS reads through a null pointer)")
    end
    return p
end

-- Appearing (0x020940E4, 0x020942B4): linked to the other; each puts on what its navi wears (`sub_8011420` with Param3
-- 1: Colonel his cape).
local function appear(me: Object)
    if me.phase_init == 0 then
        link(me)
        me:add_parts(me.identity :: Identity, 1)
    end
    double_attack.appear(me, 4, 0x10)
end

-- 0x02094140 (Colonel): the charge; then the panel ahead in row 1 his destination, and a count of 2 (+0x22, which
-- nothing reads).
local function colonel_charge(me: Object)
    if not double_attack.charge(me, CHARGE_TICKS) then
        return
    end
    me.future_panel_x, me.future_panel_y = me.panel_x + me.facing, 1
    me.timer2 = 2
    me:set_action(8)
end

-- 0x02094514: a rock on (x, y) where that panel is the other side's, solid and empty.
local function place(me: Object, x: number, y: number, behind: boolean)
    local panel = if field.valid(x, y) then field.panel(x, y) else nil
    if panel == nil or panel.alliance == me.alliance then
        return
    end
    if not field.check(x, y, SOLID, OCCUPIED) then
        return
    end
    local s = me.state :: State
    if s.rocks >= MOST_ROCKS then
        error("E-SAPPOU's Colonel drops a ninth rock: its flag lies past his eight (+0x80)")
    end
    rock.spawn(me, x, y, int.u8(s.rocks * NEXT_WAIT + FIRST_WAIT), behind, s.rocks)
    s.rocks += 1
end

-- 0x020944B0: for each panel an enemy's body stands on (0x02094568: rows 1 to 3, each from the column behind him
-- forward), a rock in front of it and one behind.
local function drop(me: Object)
    local d = me.facing
    local first = if d >= 0 then 1 else 6
    local found: { { number } } = {}
    local y = 1
    while field.valid(first, y) do
        local x = first
        repeat
            if field.check(x, y, ENEMY_BODY[me.alliance], 0) then
                table.insert(found, { x, y })
            end
            x += d
        until not field.valid(x, 1)
        y += 1
    end
    for _, p in found do
        place(me, p[1] - d, p[2], false)
        place(me, p[1] + d, p[2], true)
    end
end

-- 0x020941D8 (Colonel's drop, by phase): once ShadowMan is gone, the rocks and 60 ticks; then until they are all gone.
local function colonel_drop(me: Object)
    local s = me.state :: State
    local p = me.phase
    if p == 0 then
        if me.phase_init == 0 then
            local other = partner(me)
            if other.action ~= 8 or other.phase ~= 4 then
                return
            end
            me.anim = ANIM_DROP
            me.timer = DROP_TICKS
            drop(me)
            me.phase_init = 4
        end
        if double_attack.ran_out(me) then
            me.phase, me.phase_init = 4, 0
        end
    elseif p == 4 then
        if me.phase_init == 0 then
            me.anim = ANIM_IDLE
            me.timer = AFTER_TICKS
            me.phase_init = 4
        end
        for i = 1, MOST_ROCKS do
            if s.out[i] then
                return
            end
        end
        me:set_action(0xC)
    else
        error(string.format("E-SAPPOU's Colonel: phase %#x reads past his table", p))
    end
end

-- 0x02094394 (ShadowMan, by phase): gone in a puff of dust; back once Colonel is done.
local function shadow(me: Object)
    local p = me.phase
    if p == 0 then
        if me.phase_init == 0 then
            me.anim = ANIM_VANISH
            me.timer = VANISH_TICKS
            me.phase_init = 4
        end
        if double_attack.ran_out(me) then
            me.visible = false
            local q = me.pos
            battle.effect(Vec3.new(q.x, q.y, int.i32(q.z + DUST_Z)), effects.dust)
            me.phase, me.phase_init = 4, 0
        end
    elseif p == 4 then
        if me.phase_init == 0 then
            if partner(me).action ~= 0xC then
                return
            end
            me.visible = true
            me.anim = ANIM_BACK
            me.timer = BACK_TICKS
            battle.play_sound(SOUND_APPEAR)
            me.phase_init = 4
        end
        if double_attack.ran_out(me) then
            me:set_action(0xC)
        end
    else
        error(string.format("E-SAPPOU's ShadowMan: phase %#x reads past his table", p))
    end
end

-- 0x02094244 (Colonel: once ShadowMan is back), 0x0209441C (ShadowMan): 30 ticks, then he leaves.
local function after(me: Object, waits: boolean)
    if me.phase_init == 0 then
        if waits and partner(me).action ~= 0xC then
            return
        end
        me.timer = AFTER_TICKS
        me.phase_init = 4
    end
    if double_attack.ran_out(me) then
        me:set_action(0x10)
    end
end

local function leave(me: Object)
    double_attack.leave(me, true)
end

local chip: Chip = {
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "special",
    class = "special",
    rarity = 0,
    mb = 5,
    damage = 40,
    hit_param = 143,
    lockout = 0,
    flags = {},
    action = cannon_chips.cannon.action,
    double_attack = double_attack.make {
        id = "e-sappou",
        damage = { 45, 45, 45, 50, 60, 75, 110, 130, 150, 150 },
        state = { partner = "object", rocks = "u8", out = "bool[8]" },
        roles = {
            -- 0x020940D4's table: Colonel, (10, 10) from the user's spot.
            {
                navi = colonel,
                dx = 10,
                dy = 10,
                actions = {
                    [0x0] = appear,
                    [0x4] = colonel_charge,
                    [0x8] = colonel_drop,
                    [0xC] = function(me: Object)
                        after(me, true)
                    end,
                    [0x10] = leave,
                },
            },
            -- 0x020942A4's: ShadowMan, (-10, -10).
            {
                navi = shadowman,
                dx = -10,
                dy = -10,
                actions = {
                    [0x0] = appear,
                    [0x4] = function(me: Object)
                        if double_attack.charge(me, CHARGE_TICKS) then
                            me:set_action(8)
                        end
                    end,
                    [0x8] = shadow,
                    [0xC] = function(me: Object)
                        after(me, false)
                    end,
                    [0x10] = leave,
                },
            },
        },
    },
}

return { ["e-sappou"] = chip }
