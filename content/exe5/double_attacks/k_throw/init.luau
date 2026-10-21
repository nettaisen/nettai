--!strict
-- K-THROW (the DS's chip 0x169; Knight Throw), the Double Attack of TomahawkMan and KnightMan (docs/design/exe5ds-map.md
-- §5, §5.2: its director the DS's effect object 0xA0, its actors its T1 0x41). TomahawkMan winds up and throws;
-- KnightMan, flashing, flies in an arc three panels ahead of the user's spot and lands on that panel 119 ticks later:
-- one Wood hit there (breaking, thrown). On a solid panel he lands with a shake and cracks the 3x3 around it; on a hole
-- he goes in a puff of smoke. The chip's record and name are the DS's (written by tools/exe5/gen_content.py,
-- bn6battle-verify, from the DS ROMs); its record's use is the cannon's (action 0x14, subtype 0: the DS copied Cannon's
-- record), which nothing starts: no folder or hand holds it.
--
-- Its director links the two actors (each its partner's, +0x60), which wait on each other.

local cannon_chips = require("../chips/cannon")
local collision = require("../rules/collision")
local double_attack = require("../lib/double_attack")
local ds_arc = require("../lib/ds_arc")
local effects = require("../lib/effects")
local knightman = require("../navis/knightman").knightman
local panels = require("@exelib/panels")
local regions = require("@exelib/regions")
local sparks = require("../lib/sparks")
local tomahawkman = require("../navis/tomahawkman").tomahawkman

type State = DoubleAttackActor & { partner: Object? }

local CHARGE_TICKS = 0x3C
-- TomahawkMan's throw: his animation, 15 ticks, the throw's sound.
local ANIM_THROW, THROW_TICKS = 6, 0xF
local SOUND_THROW = asset.sound("tomahawk-throw")
-- KnightMan, thrown: 20 ticks flashing (palette 2 every other two ticks), 6 more, then in the air (animation 4) for 119
-- ticks of an arc aimed at 120, his landing animation (3) 4 ticks before the end.
local FLASH_TICKS, WIND_UP_TICKS, FLIGHT_TICKS, ARC_TICKS, LAND_ANIM_AT = 0x14, 6, 0x77, 0x78, 4
local THROWN_PALETTE = 2
local ANIM_FLY, ANIM_LAND = 4, 3
-- The arc's target: three panels ahead, 2 pixels down, 2 up; its fall a tick.
local REACH, TARGET_DY, TARGET_Z, GRAVITY = 3, 0x20000, 0x20000, -0xC000
-- The hit: element 4 (Wood), on his panel, a breaking spark, hitting navis as a thrown breaking attack, hit modifier 3.
local WOOD, HIT_MOD = 4, 3
-- On a hole: the smoke 20 pixels up. On a solid panel: the cameras' shake (3, 60 ticks), the landing's sound, the 3x3
-- cracked (`sub_80DB48A`, region 0x0F), 30 ticks.
local SMOKE_Z = 0x140000
local SHAKE, SHAKE_TICKS = 3, 0x3C
local SOUND_LAND = asset.sound("land")
local LANDED_TICKS = 0x1E
-- Both wait 30 ticks before they leave.
local AFTER_TICKS = 0x1E

-- The director's link (0x02093E8C): each actor's partner, the other actor it brought.
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
        error("K-THROW's actor has no partner (the DS reads through a null pointer)")
    end
    return p
end

-- Appearing (0x02094674, 0x02094820): linked to the other; KnightMan puts on what he wears (`sub_8011420` with Param3
-- 1: his ball and chain).
local function appear(me: Object)
    if me.phase_init == 0 then
        link(me)
        local s = me.state :: State
        if s.role == 1 then
            me:add_parts(knightman.identity :: Identity, 1)
        end
    end
    double_attack.appear(me, 4, 0x10)
end

-- 0x020946BC (TomahawkMan): the charge; then the panel ahead in row 1 his destination.
local function tomahawk_charge(me: Object)
    if not double_attack.charge(me, CHARGE_TICKS) then
        return
    end
    me.future_panel_x, me.future_panel_y = me.panel_x + me.facing, 1
    me:set_action(8)
end

-- 0x02094750 (TomahawkMan's throw, by phase): once KnightMan is up for it, the throw (15 ticks); then until KnightMan is
-- done.
local function throw(me: Object)
    local p = me.phase
    if p == 0 then
        local other = partner(me)
        if other.action == 8 and other.phase == 4 then
            me.phase, me.phase_init = 4, 0
        end
    elseif p == 4 then
        if me.phase_init == 0 then
            me.anim = ANIM_THROW
            me.timer = THROW_TICKS
            battle.play_sound(SOUND_THROW)
            me.phase_init = 4
        end
        if double_attack.ran_out(me) then
            me.phase, me.phase_init = 8, 0
        end
    elseif p == 8 then
        if partner(me).action == 0xC then
            me:set_action(0xC)
        end
    else
        error(string.format("K-THROW's TomahawkMan: phase %#x reads past his table", p))
    end
end

-- 0x02094910: KnightMan flashes for 20 ticks (palette 2 while the timer's bit 1 is set), then stays in palette 2.
local function flash(me: Object)
    if me.phase_init == 0 then
        me.timer = FLASH_TICKS
        me.phase_init = 4
    end
    me.sprite.palette = if bit32.btest(me.timer, 2) then THROWN_PALETTE else 0
    if double_attack.ran_out(me) then
        me.sprite.palette = THROWN_PALETTE
        me.phase, me.phase_init = 4, 0
    end
end

-- 0x02094944: once TomahawkMan throws, 6 ticks; then in the air, aimed three panels ahead of the user's spot.
local function wind_up(me: Object)
    if me.phase_init == 0 then
        local other = partner(me)
        if other.action ~= 8 or other.phase ~= 4 then
            return
        end
        me.timer = WIND_UP_TICKS
        me.phase_init = 4
    end
    if not double_attack.ran_out(me) then
        return
    end
    me.anim = ANIM_FLY
    local tx, ty = field.center(me.panel_x + me.facing * REACH, me.panel_y)
    me.vel = ds_arc.in_ticks(me.pos, tx, ty + TARGET_DY, TARGET_Z, ARC_TICKS, GRAVITY, me.alliance)
    me.phase, me.phase_init = 8, 0
end

-- 0x020949A8: the flight; at its end the hit on his panel, then the landing, or on a hole the smoke and gone.
local function fly(me: Object)
    if me.phase_init == 0 then
        me.timer = FLIGHT_TICKS
        me.phase_init = 4
    end
    local p, v = me.pos, me.vel
    local vz = int.i32(v.z + GRAVITY)
    me.vel = Vec3.new(v.x, v.y, vz)
    me.pos = Vec3.new(int.i32(p.x + v.x), int.i32(p.y + v.y), int.i32(p.z + vz))
    me:set_panel_from_coordinates()
    if me.timer == LAND_ANIM_AT then
        me.anim = ANIM_LAND
    end
    if not double_attack.ran_out(me) then
        return
    end
    double_attack.hit(me, {
        panel_x = me.panel_x,
        panel_y = me.panel_y,
        element = WOOD,
        z = 0,
        region = regions.single,
        hit_effect = sparks.breaking,
        target = collision.hits_navis,
        self_type = collision.thrown_break,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = HIT_MOD,
    })
    if not field.solid(me.panel_x, me.panel_y) then
        local q = me.pos
        battle.effect(Vec3.new(q.x, q.y, int.i32(q.z + SMOKE_Z)), effects.smoke)
        me.visible = false
        me:set_action(0xC)
        return
    end
    me:set_coordinates_from_panel()
    local q = me.pos
    me.pos = Vec3.new(q.x, q.y + 0x20000, TARGET_Z)
    me.phase, me.phase_init = 0xC, 0
end

-- 0x02094A4C: landed: on the ground, the shake, the sound, the 3x3 cracked; 30 ticks.
local function landed(me: Object)
    if me.phase_init == 0 then
        local p = me.pos
        me.pos = Vec3.new(p.x, p.y, 0)
        battle.shake_camera(SHAKE, SHAKE_TICKS)
        battle.play_sound(SOUND_LAND)
        panels.crack_region(me.panel_x, me.panel_y, regions.square)
        me.timer = LANDED_TICKS
        me.phase_init = 4
    end
    if double_attack.ran_out(me) then
        me:set_action(0xC)
    end
end

-- 0x02094900 (KnightMan, by phase).
local function thrown(me: Object)
    local p = me.phase
    if p == 0 then
        flash(me)
    elseif p == 4 then
        wind_up(me)
    elseif p == 8 then
        fly(me)
    elseif p == 0xC then
        landed(me)
    else
        error(string.format("K-THROW's KnightMan: phase %#x reads past his table", p))
    end
end

-- 0x020947B8, 0x02094A8C: 30 ticks, then he leaves.
local function after(me: Object)
    if me.phase_init == 0 then
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
    hit_param = 168,
    lockout = 0,
    flags = {},
    action = cannon_chips.cannon.action,
    double_attack = double_attack.make {
        id = "k-throw",
        damage = { 180, 180, 180, 180, 220, 270, 350, 400, 450, 450 },
        state = { partner = "object" },
        roles = {
            -- 0x02094664's table: TomahawkMan, on the user's spot.
            {
                navi = tomahawkman,
                dx = 0,
                dy = 0,
                actions = {
                    [0x0] = appear,
                    [0x4] = tomahawk_charge,
                    [0x8] = throw,
                    [0xC] = after,
                    [0x10] = leave,
                },
            },
            -- 0x02094810's: KnightMan, (-10, -10).
            {
                navi = knightman,
                dx = -10,
                dy = -10,
                actions = {
                    [0x0] = appear,
                    [0x4] = function(me: Object)
                        if double_attack.charge(me, CHARGE_TICKS) then
                            me:set_action(8)
                        end
                    end,
                    [0x8] = thrown,
                    [0xC] = after,
                    [0x10] = leave,
                },
            },
        },
    },
}

return { ["k-throw"] = chip }
