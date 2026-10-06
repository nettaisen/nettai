--!strict
-- A falling meteor, as EXE5 and EXE6 share it (EXE6's attack object #0x56,
-- `sub_80CF3BE`; EXE5's #0x56;
-- docs/engine/dimming-chip-effects.md §6): marked on its panel (./marker),
-- it waits 30 ticks high up behind it, falls onto it in 16, and if the
-- panel is solid or holds a body, bursts with a one-tick hit and the camera
-- shaking. Meteors (./chip) drops thirty (running while dimmed); the
-- instant chips' meteor shower (lib/instant/meteor_shower) uses it too,
-- dropping them one at a time. What it is and does when it lands is its
-- row (`byte_80CF3AC`, by the original's first parameter: EXE6's three
-- alike); each game's meteor is a kind of its look (its sprite, sounds,
-- rows, sparks, burst and marker: `falling_meteor.make_kind`).

local marker = require("./marker")
local regions = require("../regions")

local falling_meteor = {}

export type State = {
    -- Its row (Param1).
    row: number,
}



local ANIM = 0
-- It starts 256 pixels behind and above its panel, and falls 16 forward
-- and 16 down a tick.
local START, SPEED = 0x100_0000, 0x10_0000
local WAIT, FALL = 0x1E, 0x10
-- It bursts where the panel is solid or holds a body.
local BURSTS_ON = 0x0F80_0010
-- The camera shake's ticks.
local SHAKE_TICKS = 0xF

-- A meteor kind (`id`) of `look`.
function falling_meteor.make_kind(id: string, look: MeteorLook): Kind
    -- Its row.
    local function row_of(me: Object): MeteorRow
        local n = (me.state :: State).row
        local row = look.rows[n]
        if not row then
            error(string.format("a falling meteor's row %d reads past byte_80CF3AC", n))
        end
        return row
    end

    -- `sub_80CF4C8`: 30 ticks up there.
    local function wait(me: Object)
        local t = me.timer - 1
        me.timer = t
        if t ~= 0 then
            return
        end
        me.timer = FALL
        battle.play_sound(look.sounds.fall)
        me.action = 4
    end

    -- `sub_80CF4E6`: fall; on the panel, burst if it is solid or holds a body.
    local function fall(me: Object)
        local t = me.timer - 1
        me.timer = t
        if t ~= 0 then
            local p, v = me.pos, me.vel
            me.pos = Vec3.new(p.x + v.x, p.y, p.z - v.z)
            return
        end
        me:set_coordinates_from_panel()
        me.pos = Vec3.new(me.pos.x, me.pos.y, 0)
        if bit32.btest(field.flags(me.panel_x, me.panel_y), BURSTS_ON) then
            battle.effect(me.pos, look.burst)
            battle.play_sound(look.sounds.land)
            local shake = row_of(me).shake
            if shake ~= 0 then
                battle.shake_camera(shake, SHAKE_TICKS)
            end
            me.collision:set_region(regions.single)
        end
        me.action = 8
        me.visible = false
    end

    -- `sub_80CF554`: the tick after (its hit resolved): its panel cracked or
    -- broken as its row says; gone.
    local function landed(me: Object)
        local panel = row_of(me).panel
        if panel == "crack" or (panel == "crack_half" and not bit32.btest(battle.rng_positive(), 1)) then
            field.crack(me.panel_x, me.panel_y)
        elseif panel == "break" then
            field.break_panel(me.panel_x, me.panel_y)
        end
        me.visible = false
        me.collision:set_region(nil)
        me:set_lifecycle("destroy")
    end

    -- `sub_80CF488`: resolve last tick's hits, the action, present.
    local function tick(me: Object)
        me.collision:remove()
        me:hit_spark()
        local a = me.action
        if a == 0 then
            wait(me)
        elseif a == 4 then
            fall(me)
        elseif a == 8 then
            landed(me)
        else
            error(string.format("a falling meteor's action %#x reads past its table", a))
        end
        me:update_sprite()
        me.collision:present()
    end

    -- `sub_80CF3DC`: behind and above its panel, marked; its collision on no
    -- panel yet. (The update first runs next tick.)
    local function init(me: Object)
        me.visible = true
        local sprite = me.sprite
        sprite:load(look.sprite)
        -- sprite_noShadow: the first part is drawn with the sprite.
        sprite.shadow = "with_sprite"
        me.anim, me.anim_loaded = ANIM, ANIM
        sprite:set_animation(ANIM)
        sprite:step()
        sprite:set_flip(bit32.bxor(me.alliance, me.flip))
        me:set_coordinates_from_panel()
        local d = me.facing
        me.vel = Vec3.new(SPEED * d, me.vel.y, SPEED)
        me.pos = Vec3.new(me.pos.x - START * d, me.pos.y, START)
        me.timer = WAIT
        if not me:create_collision() then
            me:free()
            return
        end
        local row = row_of(me)
        me:setup_collision(row.is, row.hits, row.hit_mod)
        me.collision:set_hit_effect(look.spark(me.element))
        me.collision:set_region(nil)
        if row.bug then
            -- sub_801A4D0
            me.collision.bugs = row.bug + bit32.lshift(row.bug_arg or 0, 8)
        end
        me.collision:present()
        marker.spawn(look.marker, me, me.panel_x, me.panel_y, marker.meteor, regions.single)
        me.lifecycle = "update"
    end

    return new.kind {
        id = id,
        pool = "attack",
        state = { row = "u8" },
        -- `sub_80CF3BE`.
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                init(me)
            elseif l == "update" then
                tick(me)
            else
                me:destroy()
            end
        end,
    }
end

-- `sub_80CF594`: a meteor of `kind` and row `row` on (x, y) with `element`
-- and the damage word, of `owner`'s side and facing; `while_dimmed`
-- (`sub_80CF5B2`, Meteors'): it runs while dimmed. Its position is `pos`
-- (the spawner's registers) until its init places it.
function falling_meteor.spawn_kind(
    kind: Kind,
    row: number,
    owner: Object,
    x: number,
    y: number,
    element: number,
    damage: number,
    while_dimmed: boolean,
    pos: Vec3?
): Object?
    local o = battle.spawn(kind, pos)
    if not o then
        return nil
    end
    (o.state :: State).row = row
    o.panel_x, o.panel_y = x, y
    o.element = element
    o:set_damage_word(damage)
    o.alliance, o.flip = owner.alliance, owner.flip
    if while_dimmed then
        o.run_while_dimmed = true
    end
    return o
end

return falling_meteor
