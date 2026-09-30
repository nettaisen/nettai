--!strict
-- TomahawkMan, the navi of the TmhkMan chips (actor object #0x0A,
-- `sub_80B97C0`), brought by their navi chip controller. He appears,
-- raises his tomahawk and brings it down on the panel ahead of him: a hit
-- region over the three columns beyond (region 0x11), a splash of earth
-- (effect 0x33) and a camera shake. Then he leaves, and the controller
-- moves on. See docs/engine/chips.md §3.6.12.
--
-- The chips' script (navi chip subtype 8): `navi_chip` is his spawner,
-- `off_802CD5C[8]`.

export type State = {
    -- The controller waiting for him (its pointer sits in his collision
    -- data slot).
    controller: Object?,
}

local tomahawk_man = {
    state = { controller = "object" },
}

local SPRITE = "08-07"

-- Panel flags he needs to stay (as one word).
local CAN_STAND = 0x10010

-- The controller calls his spawner through r3, which his position takes:
-- its address with the Thumb bit, whose fraction his Z keeps.
local SPAWNER_ADDRESS = 0x080B999B

-- When his tomahawk lands (ticks left of his swing).
local STRIKE = 0x37

-- `sub_80B999A`: TomahawkMan for `user`, reporting to `controller`.
function tomahawk_man.navi_chip(user: Object, controller: Object, spec: NaviChipSpec): Object?
    local o = battle.spawn_kind("tomahawk-man", Vec3.new(spec.panel_y, spec.element, SPAWNER_ADDRESS), spec.params)
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = spec.panel_x, spec.panel_y
    o.element = spec.element
    o.related1 = user
    o.alliance, o.flip = user.alliance, user.flip
    o.damage = spec.damage
    o.stamina = bit32.rshift(spec.damage, 16)
    local s = o.state :: State
    s.controller = controller
    return o
end

-- `sub_80B97E4`: on his panel, on the ground (Z's whole part cleared).
local function init(me: Object)
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, bit32.band(p.z, 0xFFFF))
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.anim = 0
    me.anim_loaded = 0
    sprite:set_animation(0)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_lifecycle("update")
end

-- `sub_80B9848`: he appears; 3 ticks (Timer and Timer2 a word); on to
-- standing, or straight to leaving if his panel can't hold him.
local function appear(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.anim = 3
        battle.play_sound(0x94)
        me.visible = true
        me.timer, me.timer2 = 3, 0
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left > 0 then
        return
    end
    local stands = bit32.band(field.flags(me.panel_x, me.panel_y), CAN_STAND) == CAN_STAND
    me:set_action(if stands then 4 else 0x10)
end

-- `sub_80B989C`: 30 ticks standing.
local function stand(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.anim = 0
        me.timer, me.timer2 = 0x1E, 0
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left <= 0 then
        me:set_action(8)
    end
end

-- `sub_80B98C4`: 30 ticks raising the tomahawk.
local function raise(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.anim = 0xF
        me.timer = 0x1E
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left <= 0 then
        me:set_action(0xC)
    end
end

-- The strike: a hit region on the panel ahead running while dimmed
-- (`sub_80C53A6`: region 0x11, no hit spark), the earth splashing there
-- (16 pixels up), sound 0x10A, and the camera shakes (magnitude 1, 30
-- ticks).
local function strike(me: Object)
    local x, y = me.panel_x + me.facing, me.panel_y
    local box = battle.hitbox(me, {
        panel_x = x,
        panel_y = y,
        element = me.element,
        z = 0,
        region = 0x11,
        hit_effect = 0xFF,
        target = 0,
        self_type = 7,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = 1,
    })
    if box then
        box.run_while_dimmed = true
    end
    local cx, cy = field.center(x, y)
    battle.effect(Vec3.new(cx, cy, 0x10_0000), 0x33, bit32.bxor(me.alliance, me.flip))
    battle.play_sound(0x10A)
    battle.shake_camera(1, 0x1E)
end

-- `sub_80B98EC`: the swing, 61 ticks (to -1); the tomahawk lands at 55
-- left.
local function swing(me: Object)
    if me.phase_init == 0 then
        me.phase_init = 4
        me.anim = 0x10
        me.timer = 0x3C
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left == STRIKE then
        strike(me)
        return
    end
    if left < 0 then
        me:set_action(0x10)
    end
end

-- `sub_80B9970`: 3 ticks, then he goes and the controller moves on.
local function leave(me: Object)
    if me.phase_init == 0 then
        me.phase_init = 4
        me.anim = 4
        me.timer, me.timer2 = 3, 0
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left > 0 then
        return
    end
    local controller = (me.state :: State).controller
    if controller then
        navi_chip.navi_left(controller)
    end
    me:set_lifecycle("destroy")
end

local ACTIONS = { [0] = appear, [4] = stand, [8] = raise, [0xC] = swing, [0x10] = leave }

-- `sub_80B97C0`.
function tomahawk_man.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        local action = ACTIONS[me.action]
        if not action then
            error(string.format("TomahawkMan action %#x reads past his table (off_80B9834)", me.action))
        end
        action(me)
    else
        me:free()
    end
    me:update_sprite_while_dimmed()
end

return tomahawk_man
