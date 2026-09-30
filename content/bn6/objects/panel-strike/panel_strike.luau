--!strict
-- A panel strike (attack object #0x09, `sub_80C5DDC`): it highlights its
-- panel 4 ticks out of 8 for Param2 ticks, then shows a burst (sprite
-- 0x10/0x26) and hits the panel once; it goes when the burst's animation
-- ends. Bass's shots (navi chip subtype 26) and MachGun's (action 0x29)
-- are these.
--
-- Its parameters: Param1 set, the burst cracks an uncracked panel;
-- Param2 the ticks before it; Param3 the hit modifier (0 → 3, 1 → 1,
-- else 0); Param4 set, the burst only on a solid panel (else it just goes).
-- One that runs while dimmed (`sub_80C5F4A`) outlasts the battle's end.
-- See docs/engine/chips.md §3.6.33.

local panel_strike = {}

local SPRITE = "10-26"

-- `sub_80C5F2C` (`while_dimmed`: `sub_80C5F4A`): a strike on (x, y) with
-- `element`, the damage word and `params`, on `owner`'s side. `z` is the
-- spawner's register that becomes its Z (its init keeps it).
function panel_strike.spawn(
    owner: Object,
    x: number,
    y: number,
    element: number,
    params: { number },
    damage: number,
    z: number,
    while_dimmed: boolean
): Object?
    local o = battle.spawn_kind("panel-strike", Vec3.new(y, element, z), params)
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.damage = damage
        o.stamina = bit32.rshift(damage, 16)
        o.alliance, o.flip = owner.alliance, owner.flip
        if while_dimmed then
            o.run_while_dimmed = true
        end
    end
    return o
end

-- `sub_80C5E00`: on its panel, its collision on with its region off.
local function init(me: Object)
    me:set_coordinates_from_panel()
    me.timer = me:param(2)
    if not me:create_collision() then
        me:free()
        return
    end
    local mode = me:param(3)
    local hit_mod = if mode == 0 then 3 elseif mode == 1 then 1 else 0
    me:setup_collision(0x0A, 5, hit_mod)
    local c = me.collision
    c.hit_effect = 0
    c.region = 0
    c:present()
    me:set_lifecycle("update")
end

-- `sub_80C5E84`: the highlight, then the burst and its hit (and a crack),
-- or, where Param4 wants a solid panel and there is none, nothing.
local function wait(me: Object)
    if bit32.band(me.timer, 4) == 0 then
        field.highlight(me.panel_x, me.panel_y)
    end
    local left = me.timer - 1
    me.timer = left
    if left > 0 then
        return
    end
    local c = me.collision
    if me:param(4) ~= 0 and not field.solid(me.panel_x, me.panel_y) then
        c.region = 0
        me:set_lifecycle("destroy")
        return
    end
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim = 0
    me.anim_loaded = 0
    sprite:set_animation(0)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    c.region = 1
    if me:param(1) ~= 0 then
        -- sub_8109794: crack it unless it is cracked already.
        if bit32.band(field.flags(me.panel_x, me.panel_y), 0x40) == 0 then
            field.crack(me.panel_x, me.panel_y)
        end
    end
    battle.play_sound(0xB9)
    me:set_action(4)
end

-- `sub_80C5F0C`: the region off; gone when the burst ends.
local function burst(me: Object)
    if me.phase == 0 then
        me.phase = 1
        me.collision.region = 0
    end
    if bit32.band(me.sprite.frame_flags, 0x80) ~= 0 then
        me:set_lifecycle("destroy")
    end
end

local ACTIONS = { [0] = wait, [4] = burst }

-- `sub_80C5E44`: resolve the hits, act, register again; gone once the
-- battle is over (unless it runs while dimmed).
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if not me.run_while_dimmed and battle.over() then
        c.region = 0
        me:set_lifecycle("destroy")
        return
    end
    local action = ACTIONS[me.action]
    if not action then
        error(string.format("panel strike action %#x reads past its table (off_80C5E7C)", me.action))
    end
    action(me)
    c:present()
end

-- `sub_80C5DDC`.
function panel_strike.update(me: Object)
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

return panel_strike
