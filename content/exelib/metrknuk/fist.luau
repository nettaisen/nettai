--!strict
-- A MetrKnuk fist, as EXE6 and EXE5 share it (EXE6's attack object #0xB4,
-- `sub_80DBCEC`; EXE5's #0x76, 0x080D4C58; docs/engine/dimming-chip-effects.md
-- §18): it falls from high up behind its panel onto it, hits it with a
-- burst, cracks it and shakes the camera, then bounces back up and goes.
-- EXE5's falls without a sound, and goes when the battle ends. Each game
-- makes its kind (`fist.make`: its kind and its spawn).

local regions = require("../regions")

local fist = {}

-- A game's fists.
export type Look = {
    id: string,
    sprite: SpriteAsset,
    -- The sound it falls with (EXE6's; EXE5's falls without one).
    sound: Sound?,
    -- Its landing's burst, and its hit: what it is and hits, its spark.
    burst: Effect,
    self_type: CollisionType,
    target: CollisionType,
    spark: Spark,
    -- EXE5's: it goes when the battle ends (0x080D4D08).
    ends_with_battle: boolean,
}

export type State = {
    -- The fast fall (Param2 4, what MetrKnuk's spawner sets) or the slow
    -- one (Param2 0, which no spawner leaves).
    fast: boolean,
}

-- Where it starts: 134 pixels behind its panel's center, 16 down, at the
-- spawner's height.
local BEHIND, DOWN = 0x86_0000, 0x10_0000
-- By Param2 (0, 4): the fall's first speed, the speed forward and the
-- gravity (EXE6's `byte_80DBD8C`, `byte_80DBD98`, `byte_80DBE44`).
local FALL: { slow: number, fast: number } = { slow = 0x6_0000, fast = 0xE_0000 }
local FORWARD: { slow: number, fast: number } = { slow = 0xA_0000, fast = 0x11_8000 }
local GRAVITY: { slow: number, fast: number } = { slow = 0x7800, fast = 0xD200 }
-- Its hit (`sub_80DBEBA`, 0x15050001): on its panel, hit modifier 3.
local HIT_MOD = 3
-- The bounce: 7 ticks, moving back for the last 4.
local BOUNCE, BOUNCE_MOVES = 7, 4
local CRACKED, SOLID = 0x40, 0x10

-- A game's fist kind, and its spawn.
function fist.make(look: Look): {
    kind: Kind,
    spawn: (spawner: Object, x: number, y: number, element: number, z: number, damage: number) -> Object?,
}
    local function row(me: Object, t: { slow: number, fast: number }): number
        return if (me.state :: State).fast then t.fast else t.slow
    end

    -- `sub_80DBDBC`: forward and down; on landing a burst (jittered), the hit,
    -- the panel cracked (the slow one cracks a panel not yet cracked only on
    -- an even draw), and on a solid panel the camera shaken.
    local function fall(me: Object)
        local p = me.pos
        local x = p.x + me.vel.x
        local vz = me.vel.z + row(me, GRAVITY)
        me.vel = Vec3.new(me.vel.x, me.vel.y, vz)
        local z = int.i32(p.z - vz)
        if z >= 0 then
            me.pos = Vec3.new(x, p.y, z)
            return
        end
        me.pos = Vec3.new(x, p.y, p.z)
        me.action, me.phase, me.phase_init = 4, 0, 0
        local px, py = me.panel_x, me.panel_y
        local cx, cy = field.center(px, py)
        battle.effect(battle.jitter(3, Vec3.new(cx, cy, 0)), look.burst)
        local h = battle.hitbox(me, {
            panel_x = px,
            panel_y = py,
            element = me.element,
            z = p.z,
            region = regions.single,
            hit_effect = look.spark,
            target = look.target,
            self_type = look.self_type,
            damage = me.damage,
            stamina = me.stamina,
            hit_mod = HIT_MOD,
        })
        if h then
            h.run_while_dimmed = me.run_while_dimmed
        end
        local flags = field.flags(px, py)
        if (me.state :: State).fast
            or (not bit32.btest(flags, CRACKED) and not bit32.btest(battle.rng_positive(), 1))
        then
            field.crack(px, py)
        end
        if bit32.btest(flags, SOLID) then
            battle.shake_camera(1, 5)
        end
        me.pos = Vec3.new(x, p.y, 0)
    end

    -- `sub_80DBE4C`: 7 ticks (the first sets the count), the last 4 moving
    -- back up the way it came; then gone.
    local function bounce(me: Object)
        if me.phase_init == 0 then
            me.phase_init = 1
            me.timer = BOUNCE
            return
        end
        if me.timer <= BOUNCE_MOVES then
            local p = me.pos
            me.pos = Vec3.new(p.x - me.vel.x, p.y, p.z + me.vel.z)
        end
        local t = me.timer - 1
        me.timer = t
        if t == 0 then
            me:set_lifecycle("destroy")
        end
    end

    -- `sub_80DBD10`: behind its panel, its speeds, its look, (EXE6's) its
    -- sound. (Its update first runs next tick.)
    local function init(me: Object)
        me:set_coordinates_from_panel()
        local p = me.pos
        me.pos = Vec3.new(p.x - me.facing * BEHIND, p.y + DOWN, p.z)
        me.vel = Vec3.new(me.facing * row(me, FORWARD), me.vel.y, row(me, FALL))
        local sprite = me.sprite
        sprite:load(look.sprite)
        sprite.shadow = "with_sprite"
        me.visible = true
        me.anim, me.anim_loaded = 0, 0
        sprite:set_animation(0)
        sprite.palette = 0
        sprite:set_flip(bit32.bxor(me.alliance, me.flip))
        local sound = look.sound
        if sound then
            battle.play_sound(sound)
        end
        me:set_lifecycle("update")
    end

    local kind = new.kind {
        id = look.id,
        pool = "attack",
        state = { fast = "bool" },
        -- `sub_80DBCEC`.
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                init(me)
            elseif l == "update" then
                if look.ends_with_battle and battle.over() then
                    me:set_lifecycle("destroy")
                else
                    local a = me.action
                    if a == 0 then
                        fall(me)
                    elseif a == 4 then
                        bounce(me)
                    else
                        error(string.format("MetrKnuk's fist: action %#x reads past off_80DBDB4", a))
                    end
                end
            else
                me:destroy()
                return
            end
            me:update_sprite()
        end,
    }

    -- `sub_80DBEA2` (`sub_80DBE82` and the fast fall): a fist over (`x`, `y`)
    -- of `spawner`'s side and facing, at height `z`, running while dimmed.
    -- (Its X and Y are the spawner's registers until its init.)
    local function spawn(spawner: Object, x: number, y: number, element: number, z: number, damage: number): Object?
        local o = battle.spawn(kind, Vec3.new(0, 0, z))
        if o then
            o.panel_x, o.panel_y = x, y
            o.element = element
            o:set_damage_word(damage)
            o.alliance, o.flip = spawner.alliance, spawner.flip
            o.related1 = spawner
            o.run_while_dimmed = true
            (o.state :: State).fast = true
        end
        return o
    end

    return { kind = kind, spawn = spawn }
end

return fist
