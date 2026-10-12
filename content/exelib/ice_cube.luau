--!strict
-- The ice cube, as EXE6 and EXE4 share it (the original's attack object #0xA3: EXE6's `sub_80D984C`, EXE4's
-- 0x080E13FC; docs/engine/field-objects.md §4.5): an aqua obstacle that bursts when a body or another obstacle touches
-- it and, pushed, slides toward its enemy 10 pixels a tick until it leaves solid ground. It forms on its panel (its
-- look 0: its sprite's animation 0, then 2) or stands formed at once (look 1: animation 2). What an obstacle encased in
-- a bubble becomes (the role `hooks.encased`, `replace`: EXE6's content calls it the encased bubble, by that flag), and
-- in EXE4 what ColdMan forms and punches. EXE6's code and EXE4's are the same but for the parts each game passes
-- (`ice_cube.make`): its sprite, sounds, effects, debris and spark, what pushes it (its final hit modifier's bits:
-- EXE6's 0x3C, EXE4's 0xFC) and the ice block an obstacle encased in ice becomes.

local rock_debris = require("@exelib/rock/debris")

local ice_cube = {}

export type Look = {
    id: string,
    sprite: SpriteAsset,
    -- As it forms (after its first tick) and as it bursts.
    sounds: { form: Sound, burst: Sound },
    -- Its hit spark (the aqua one: hit effect 2) and its collision types (a neutral obstacle that everything reaches,
    -- rows 0x0E and 0x0F), hit modifier 3.
    spark: Spark,
    is: CollisionType,
    hits: CollisionType,
    -- Its final hit modifier's bits that push it.
    pushes: number,
    -- The debris its burst throws up (palette 1), the effect it leaves (16 pixels up), and the puff it goes in off
    -- solid ground (12 pixels up).
    debris: Kind,
    effects: { burst: Effect, puff: Effect },
    -- The game's rock, which an obstacle encased in ice becomes (its ice block variant).
    ice_block: { spawn: (x: number, y: number, side: number, spec: RockSpec, damage: number) -> Object?, variant: RockVariant },
}

export type State = {
    -- Its look (the original's first parameter): 0 forming, then 2 (formed); 1 formed from the start.
    look: number,
}

export type IceCube = {
    kind: Kind,
    -- `sub_80D99EC` (EXE4's 0x080E159C): a cube of `look` on (x, y), aqua, with `owner`'s side and the damage word,
    -- running while dimmed. Its position is the spawner's registers (the row, the side and `z`), which its init
    -- overwrites but for Z.
    spawn: (owner: Object, x: number, y: number, look: number, damage: number, z: number) -> Object?,
    -- The role `hooks.encased` (`sub_801813A`'s end; EXE4's 0x08011788): an obstacle encased in ice becomes an ice
    -- block (the rock's ice variant, in the same registry class, there at once), one in a bubble the cube, formed; at
    -- its panel, with its side and damage word.
    replace: (obstacle: Object, ice: boolean, class: number?) -> (),
}

local SPEED = 0xA_0000
local FORMED = 2
-- What bursts it: bodies and neutral objects.
local BURSTS_ON = 0x0C80_0000
local HIT_MOD = 3
local SOLID = 0x10
local AQUA = 2

function ice_cube.make(look: Look): IceCube
    -- `sub_80D99A4`: burst: two debris, the effect, the sound; gone.
    local function burst(me: Object)
        local p = me.pos
        rock_debris.spawn(look.debris, battle.jitter(0xF, p), 1)
        rock_debris.spawn(look.debris, battle.jitter(0xF, p), 1)
        battle.effect(Vec3.new(p.x, p.y, p.z + 0x10_0000), look.effects.burst)
        battle.play_sound(look.sounds.burst)
        me.visible = false
        me:set_lifecycle("destroy")
    end

    -- `sub_80D9930`: formed (after its forming animation for look 0), it waits for a push.
    local function sit(me: Object)
        local s = me.state :: State
        if s.look == 0 then
            if not bit32.btest(me.sprite.frame_flags, 0x80) then
                return
            end
            me.anim = FORMED
            s.look = FORMED
        end
        if bit32.btest(me.collision.hit_mod_final, look.pushes) then
            me:set_action(4)
        end
    end

    -- `sub_80D995C`: slide; off solid ground, gone in a puff.
    local function slide(me: Object)
        local p = me.pos
        me.pos = Vec3.new(int.i32(p.x + me.vel.x), p.y, p.z)
        -- sub_800E258: the panel under it, taken only if solid.
        local ox, oy = me.panel_x, me.panel_y
        me:set_panel_from_coordinates()
        if bit32.btest(field.flags(me.panel_x, me.panel_y), SOLID) then
            me:update_collision_panels()
            return
        end
        me.panel_x, me.panel_y = ox, oy
        me.collision:set_region(nil)
        battle.effect(Vec3.new(me.pos.x, me.pos.y, me.pos.z + 0xC_0000), look.effects.puff)
        me.visible = false
        me:set_lifecycle("destroy")
    end

    local ACTIONS: { [number]: (Object) -> () } = { [0] = sit, [4] = slide, [8] = burst }

    -- `sub_80D98E8`: resolve last tick's hits; a body's or an obstacle's touch bursts it; else its action; off the
    -- field, it bursts.
    local function tick(me: Object)
        local c = me.collision
        c:remove()
        me:hit_spark()
        if not bit32.btest(c.hit_flags, BURSTS_ON) then
            local action = ACTIONS[me.action]
            if not action then
                error(string.format("the ice cube's action %#x reads past its table (off_80D9924)", me.action))
            end
            action(me)
            if field.valid(me.panel_x, me.panel_y) then
                c:present()
                return
            end
        end
        c:set_region(nil)
        burst(me)
    end

    -- `sub_80D9870`: on its panel, sliding toward the enemy when pushed. On at once.
    local function init(me: Object)
        local s = me.state :: State
        local sprite = me.sprite
        sprite:load(look.sprite)
        -- sprite_noShadow: the first part is drawn with the sprite.
        sprite.shadow = "with_sprite"
        me.visible = true
        me.anim, me.anim_loaded = s.look * 2, s.look * 2
        sprite:set_animation(s.look * 2)
        sprite:set_flip(bit32.bxor(me.alliance, me.flip))
        me.vel = Vec3.new(SPEED * (1 - 2 * me.alliance), me.vel.y, me.vel.z)
        me:set_coordinates_from_panel()
        if not me:create_collision() then
            me:free()
            return
        end
        me:setup_collision(look.is, look.hits, HIT_MOD)
        me.collision:set_hit_effect(look.spark)
        me.collision:present()
        me:set_lifecycle("update")
        tick(me)
        battle.play_sound(look.sounds.form)
    end

    local kind = new.kind {
        id = look.id,
        pool = "attack",
        state = { look = "u8" },
        -- `sub_80D984C`.
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                init(me)
            elseif l == "update" then
                tick(me)
            else
                me:destroy()
                return
            end
            if me.active then
                me:update_sprite()
            end
        end,
    }

    local function spawn(owner: Object, x: number, y: number, cube_look: number, damage: number, z: number): Object?
        local o = battle.spawn(kind, Vec3.new(y, owner.alliance, z))
        if not o then
            return nil
        end
        (o.state :: State).look = cube_look
        o.panel_x, o.panel_y = x, y
        o.element = AQUA
        o:set_damage_word(damage)
        o.alliance = owner.alliance
        o.run_while_dimmed = true
        return o
    end

    -- (The cube's Z is the spawner's r3: what `sub_80E544C` last read, side 1's wind object: 0 when side 1 has none,
    -- or the encased obstacle was it; otherwise an address, which the port can't give: 0 too, unverified. An
    -- unregistered obstacle's ice block would take class 0xFF, which `setFieldBattleObject_800F614` turns into a
    -- write past the registry: a content error here.)
    local function replace(obstacle: Object, ice: boolean, class: number?)
        local damage = bit32.bor(obstacle.damage, bit32.lshift(obstacle.stamina, 16))
        if ice then
            if class == nil then
                error("an unregistered obstacle's ice block registers in class 0xFF, past the field-object registry")
            end
            local block: RockSpec = { variant = look.ice_block.variant, class = class, entrance = "instant" }
            look.ice_block.spawn(obstacle.panel_x, obstacle.panel_y, obstacle.alliance, block, damage)
        else
            spawn(obstacle, obstacle.panel_x, obstacle.panel_y, 1, damage, 0)
        end
    end

    return { kind = kind, spawn = spawn, replace = replace }
end

return ice_cube
