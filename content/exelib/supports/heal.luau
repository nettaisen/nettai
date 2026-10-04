--!strict
-- Tango's heal, as BN5 and BN6 share it (BN6's attack object #0xC7, `sub_80DE000`; BN5's #0x9F, 0x080DA6AC, the same
-- code), of a game's `TangoHealLook` (`heal.make`): thrown by Tango (./tango), it arcs onto her navi's panel in 20
-- ticks; landed, it tells her (her `healing` flag), leaves a sparkle, heals her navi 300 HP and gives it the game's
-- 100-HP barrier with the barrier's visual (`raise_barrier`). See docs/engine/chips.md §2.10.

export type State = {
    -- The panel it lands on (the original's third and fourth parameters).
    target_x: number,
    target_y: number,
}

local heal = {}

local FLIGHT_TICKS = 0x14
-- The arc's gravity (Z velocity, 16.16 a tick: 0xFFFF7778).
local GRAVITY = -0x8888
local HEAL = 300

-- A game's heal, of `look`.
function heal.make(look: TangoHealLook): TangoHeal
    -- `sub_80DE024` (BN5's 0x080DA6D0): its look, and the arc onto the panel.
    local function init(me: Object)
        me.visible = true
        local sprite = me.sprite
        sprite:load(look.sprite)
        sprite.shadow = "with_sprite"
        me.anim, me.anim_loaded = 0, 0
        sprite:set_animation(0)
        sprite:step()
        sprite:set_flip(bit32.bxor(me.alliance, me.flip))
        local s = me.state :: State
        local x, y = field.center(s.target_x, s.target_y)
        me.vel = look.trajectory.in_ticks(me.pos, x, y, 0, FLIGHT_TICKS, GRAVITY)
        me.timer = FLIGHT_TICKS
        me.lifecycle = "update"
    end

    -- `sub_80DE088` (BN5's 0x080DA734): the arc; landed, the heal.
    local function tick(me: Object)
        if bit32.band(me.sprite.frame_flags, 0x80) ~= 0 then
            me.anim, me.anim_loaded = 1, 0
        end
        local t = me.timer - 1
        me.timer = int.u16(t)
        if t >= 0 then
            local p, v = me.pos, me.vel
            me.pos = Vec3.new(int.i32(p.x + v.x), int.i32(p.y + v.y), int.i32(p.z + v.z))
            me.vel = Vec3.new(v.x, v.y, int.i32(v.z + GRAVITY))
            me:set_panel_from_coordinates()
            return
        end
        local tango = me.related1 :: Object
        local her = tango.state :: { healing: boolean }
        her.healing = false
        me:set_lifecycle("destroy")
        battle.effect(me.pos, look.sparkle)
        local navi = battle.player(me.alliance)
        if not navi then
            -- The game heals and shields through a null pointer.
            error("Tango's heal has no navi to heal (sub_80DE088 reads a null object)")
        end
        navi.hp = math.min(navi.hp + HEAL, navi.max_hp)
        battle.play_sound(look.sound)
        -- Barrier type 5 (BN6's `sub_801A7CC`, BN5's 0x080174DA), then the barrier's visual: the navi's old one (its
        -- AIData+0x60) goes (`sub_80E0DC0`) and a new one comes (effect object #7, `sub_80E0D98`).
        look.raise_barrier(navi)
    end

    -- `sub_80DE000` (BN5's 0x080DA6AC): by state, then its sprite.
    local function update(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
        end
        me:update_sprite()
    end

    local kind = define.kind {
        id = look.id,
        pool = "attack",
        state = { target_x = "u8", target_y = "u8" },
        update = update,
    }

    -- `sub_80DE112` (BN5's 0x080DA7BE): the heal from `tango` at `pos`, for her navi's panel (x, y).
    local function spawn(tango: Object, pos: Vec3, x: number, y: number): Object?
        local o = battle.spawn(kind, pos)
        if not o then
            return nil
        end
        local s = o.state :: State
        s.target_x, s.target_y = x, y
        o.alliance = tango.alliance
        o.related1 = tango
        local her = tango.state :: { healing: boolean }
        her.healing = true
        o.run_while_dimmed = true
        return o
    end

    return { kind = kind, spawn = spawn }
end

return heal
