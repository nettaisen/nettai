--!strict
-- A flame of the hook (./hook), as EXE6 and EXE5 share it (EXE6's attack
-- object #0xCA, `sub_80DE7F4`, FlmHook's and BigHook's; EXE5's #0xCA,
-- 0x080DF9D4, BigHook's): it swings along an arc of 160 pixels' radius
-- around a point behind its column and 48 down, starting at an angle and
-- lasting as long as its table says (by kind, facing and swing). It hits
-- the two panels ahead of where it is, only on the tick it enters a panel;
-- a hit flashes the screen (EXE5's shakes it too). It faces its object's
-- way (the attack setup's last step, EXE6's `sub_8011504` and EXE5's
-- 0x0800F1A0: a side 1 user's is drawn turned around). EXE6's leave
-- afterimages, take a big one's palette, and center their arcs from their
-- columns; EXE5's (the big kind alone) do none of that, their centers set
-- by swing. Each game makes its kind (`fire.make`: its kind and its swing);
-- each chip states its flames (`fire.variant`).

local regions = require("../regions")

local fire = {}

-- What a flame is swung with: which way it turns (`swing` 0 or 1, its
-- Param1), its chip's variant (Param2's rows), its hit spark (EXE6's
-- Param3).
export type Swing = { swing: number, variant: Variant, spark: Spark }

-- A chip's flames (`FlameHookVariant`).
export type Variant = FlameHookVariant

function fire.variant(v: Variant): Variant
    return new.record("flmhook-fire-variant", v)
end

-- A game's flames.
export type Look = {
    id: string,
    sprite: SpriteAsset,
    sound: Sound,
    self_type: CollisionType,
    target: CollisionType,
    -- Its arc's center's x (16.16), from its facing, swing and column.
    center_x: (me: Object, swing: number, turn: number) -> number,
    -- EXE5's: a hit shakes the camera too.
    hit_shakes: boolean,
}

export type State = {
    swing: number,
    variant: Variant,
    spark: Spark?,
    -- The arc's center (ExtraVars, ExtraVars+4), 16.16.
    center_x: number,
    center_y: number,
    -- Ticks since its last afterimage (ExtraVars+8).
    trail: number,
}

-- Its hit modifier (EXE6's `dword_80DEA38`).
local HIT_MOD = 3
-- Where the arc's center is down, 16.16.
local CENTER_DOWN = 0x30_0000
-- The arc's radius (EXE6's `byte_80DEA44`), 16.16 per unit of the sine
-- table.
local RADIUS = 0xA000
-- A hit's screen flash: 10 ticks, going on while dimmed; EXE5's shake.
local FLASH_TICKS = 10
local SHAKE_MAGNITUDE, SHAKE_TICKS = 3, 0x14

-- A game's flame kind, over its game's sine table, and its swing.
function fire.make(look: Look, sine: (angle: number) -> number): {
    kind: Kind,
    swing: (owner: Object, x: number, y: number, element: number, damage: number, how: Swing) -> Object?,
}
    -- `sub_80DE9E4`: an afterimage where it is (its sprite; a shadow at its
    -- height; its variant's palette, `sub_80E3428`).
    local function afterimage(me: Object)
        local v = (me.state :: State).variant
        local trail = v.trail
        if not trail then
            return
        end
        battle.afterimage(me, me.pos, {
            sprite = look.sprite,
            anim = me.anim,
            flip = bit32.bxor(me.alliance, me.flip),
            lifetime = trail.ticks,
            color_shader = trail.shader,
            palette = v.palette,
        })
    end

    -- `sub_80DE984`: onto the arc at its angle (the timer's high byte), turn
    -- the angle, and count its life down; what is left.
    local function swing(me: Object, s: State): number
        local angle = bit32.rshift(me.timer, 8)
        local c, sn = sine(angle + 64), sine(angle)
        local p = me.pos
        me.pos = Vec3.new(int.mul32(RADIUS, c) + s.center_x, int.mul32(RADIUS, sn) + s.center_y, p.z)
        -- The game reads the swing and Param2 as one halfword here: a big
        -- one turns hundreds of times further, wrapping.
        local way = (s.swing + s.variant.swing_high * 0x100) * 2 - 1
        me.timer = me.timer + s.variant.turn * way * me.facing
        local left = me.timer2 - 1
        me.timer2 = left
        return left
    end

    -- `sub_80DE924`.
    local function burn(me: Object, s: State)
        local trail = s.variant.trail
        if trail then
            if s.trail == 1 then
                afterimage(me)
            end
            local n = s.trail + 1
            if n >= trail.every then
                n = 0
            end
            s.trail = n
        end
        if swing(me, s) == 0 then
            me:set_lifecycle("destroy")
            me.collision:set_region(nil)
        end
        me:set_panel_from_coordinates()
        me:update_collision_panels()
    end

    -- `sub_80DE8C0`.
    local function tick(me: Object)
        local s = me.state :: State
        local c = me.collision
        c:remove()
        me:hit_spark()
        c:set_region(nil)
        if c.hit_flags ~= 0 then
            if look.hit_shakes then
                battle.shake_camera(SHAKE_MAGNITUDE, SHAKE_TICKS)
            end
            -- A white flash that goes on while dimmed.
            battle.palette_flash(0, FLASH_TICKS, true)
        end
        local x, y = me.panel_x, me.panel_y
        local a = me.action
        if a ~= 0 then
            error(string.format("The hook's flame action %#x reads past its table", a))
        end
        burn(me, s)
        if me.lifecycle ~= "destroy" and (me.panel_x ~= x or me.panel_y ~= y) and field.valid(me.panel_x, me.panel_y) then
            c:set_region(regions.long)
        end
        me:highlight_collision_panels()
        c:present()
    end

    -- `sub_80DE818`.
    local function init(me: Object)
        local s = me.state :: State
        local turn = bit32.bxor(me.alliance, me.flip)
        -- (The canonical side's facing, unturned: its row is the one read as it is.)
        local canonical = field.canonical_side()
        local arc = s.variant.arcs
        local at = canonical * 4 + s.swing + 1
        if arc[at + 2] == nil then
            error(string.format("The hook's flame swing %d reads past its arc table", s.swing))
        end
        -- The angle (the timer's high byte) and the ticks, from the table's row by the facing. The original's row for
        -- a flame facing left isn't the mirror image of the other everywhere (BigHook's second swing starts at 0x8A
        -- where the mirror of 0xD2 is 0xAE), and its angle, turning the other way, rounds the other way. nettai
        -- swings a flame facing the other way than the canonical side's as the mirror image of one facing its way:
        -- that row's angle mirrored, its low byte full so that it rounds as that row's does
        -- (docs/engine/side-symmetry.md).
        if turn == canonical then
            me.timer = bit32.lshift(arc[at], 8)
        else
            me.timer = bit32.bor(bit32.lshift(bit32.band(128 - arc[at], 0xFF), 8), 0xFF)
        end
        me.timer2 = arc[at + 2]
        s.center_x = look.center_x(me, s.swing, turn)
        s.center_y = CENTER_DOWN
        me.visible = true
        -- sub_8011504 (EXE5's 0x0800F1A0): its sprite, its collision, and
        -- its flip its object's.
        local sprite = me.sprite
        sprite:load(look.sprite)
        sprite.shadow = "with_sprite"
        me.anim, me.anim_loaded = if s.swing ~= 0 then 1 else 0, 0xFF
        if not me:create_collision() then
            -- The game goes on without one, writing through a null pointer.
            error("The hook's flame without a collision slot (sub_80DE818 doesn't check)")
        end
        me:setup_collision(look.self_type, look.target, HIT_MOD)
        sprite:set_flip(turn)
        -- EXE6's `sprite_setPalette` (EXE5 sets none: its variant's is 0,
        -- a loaded sprite's).
        sprite.palette = s.variant.palette
        local c = me.collision
        c:set_hit_effect(s.spark :: Spark)
        c:set_region(nil)
        battle.play_sound(look.sound)
        me:set_lifecycle("update")
        tick(me)
    end

    local kind = new.kind {
        id = look.id,
        pool = "attack",
        state = { swing = "u8", variant = "record:flmhook-fire-variant", spark = "spark", center_x = "i32", center_y = "i32", trail = "u32" },
        -- `sub_80DE7F4`.
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                init(me)
            elseif l == "update" then
                tick(me)
            else
                me:destroy()
            end
            me:update_sprite()
        end,
    }

    -- `sub_80DE960` and `sub_801155A`: a flame `owner`'s (the hook) on
    -- (x, y), running while dimmed if its variant says. (It spawns at the
    -- registers' values, which its update replaces.)
    local function swing_from(owner: Object, x: number, y: number, element: number, damage: number, how: Swing): Object?
        local o = battle.spawn(kind, Vec3.new(y, element, 0))
        if o then
            o.panel_x, o.panel_y = x, y
            o.element = element
            o:set_damage_word(damage)
            o.alliance, o.flip = owner.alliance, owner.flip
            o.related1 = owner
            local s = o.state :: State
            s.swing, s.variant, s.spark = how.swing, how.variant, how.spark
            if how.variant.while_dimmed then
                o.run_while_dimmed = true
            end
        end
        return o
    end

    return { kind = kind, swing = swing_from }
end

return fire
