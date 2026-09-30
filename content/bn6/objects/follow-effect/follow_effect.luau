--!strict
-- A follow effect (effect object #0x31, `sub_80E3D68`): a sprite shown for
-- its owner (related1), whose visibility it copies; Param1 picks its look
-- and what it does (`off_80E3E10`): 0 to 2 nothing more; 3 slides out
-- beside its owner until the owner's related1 is gone, then plays out; 4
-- (DeltaRay's) shows for its Timer (60 ticks, which its spawner sets
-- again), switching to animation 1 below 55; 5 to 8 last while their owner is in
-- action 0xB (5, 8: 30 ticks at most), 0xD (6) or 0xA (7), 6 and 7 riding
-- on it; 5 to 8 freeze while dimmed. Param2 is its flip, Param3 added to its
-- palette. Purely visual, but it holds an effect slot. See
-- docs/engine/chips.md §3.6.36.

local follow_effect = {}

type Look = { sprite: SpriteId, anim: number, palette: number }

local function look(sprite: SpriteId, anim: number): Look
    return { sprite = sprite, anim = anim, palette = 0 }
end

-- By Param1 (`byte_80E3D44`): sprite, animation and palette.
local LOOKS: { [number]: Look } = {
    [0] = look("10-00", 0),
    look("10-00", 0),
    look("10-00", 0),
    look("0C-00", 2),
    look("10-0E", 0),
    look("10-11", 0),
    look("0C-38", 0xE),
    look("0C-38", 0xF),
    look("10-4A", 1),
}

-- Variant 3's slide, pixels a tick (`dword_80E3EB0`).
local SLIDE = { [0] = 2, 3, 2, 0 }

-- `sub_80E3FB4`: a follow effect for `owner` at `pos` with `params`
-- {variant, flip, palette add}.
function follow_effect.spawn(owner: Object, pos: Vec3, params: { number }): Object?
    local o = battle.spawn_kind("follow-effect", pos, params)
    if o then
        o.related1 = owner
    end
    return o
end

local function owner_action(me: Object): number
    return (me.related1 :: Object).action
end

-- The variants 5 to 8 hold their sprite while dimmed (HP 1 as a flag).
local function frozen(me: Object): boolean
    me.hp = 1
    if battle.dimmed() then
        return true
    end
    me.hp = 0
    return false
end

-- `sub_80E3E38`: slide out beside the owner (16 pixels ahead or behind by
-- Param2), until the owner's related1 is gone; then play out. (On its first
-- tick the game compares what the sound call left in r0 with 1; the port
-- takes it to be another value, so that tick only plays out.)
local function slide(me: Object)
    local first = me.action == 0
    if first then
        me.action = 1
        me.timer2 = 0
        battle.play_sound(0xA0)
    end
    if first or me.action ~= 1 then
        if bit32.band(me.sprite.frame_flags, 0x80) ~= 0 then
            me:set_lifecycle("destroy")
        end
        return
    end
    local owner = me.related1
    if owner then
        local dx = if me:param(2) ~= 0 then -0x10 else 0x10
        local p, q = me.pos, owner.pos
        me.pos = Vec3.new(q.x + dx * 0x10000, q.y, p.z)
    end
    local t = me.timer2
    if t ~= 4 then
        local step = SLIDE[t]
        if me:param(2) == 0 then
            step = -step
        end
        local p = me.pos
        me.pos = Vec3.new(p.x + step * 0x10000, p.y, p.z)
        me.timer2 = t + 1
    end
    if (me.related1 :: Object).related1 == nil then
        me.anim = 3
        me.action = 2
    end
end

-- `sub_80E3EB4`: shows for its Timer, animation 1 below 55.
local function flash(me: Object)
    if me.action == 0 then
        me.action = 4
        me.timer = 0x3C
    end
    if me.timer < 0x37 then
        me.anim = 1
    end
    local left = me.timer - 1
    me.timer = left
    if left <= 0 then
        me:set_lifecycle("destroy")
    end
end

-- `sub_80E3EDC`: while the owner is in action 0xB.
local function while_b(me: Object)
    if frozen(me) then
        return
    end
    if owner_action(me) ~= 0xB then
        me:set_lifecycle("destroy")
    end
end

local function ride(me: Object)
    me.pos = (me.related1 :: Object).pos
end

-- `sub_80E3EFC`: riding on the owner while it is in action 0xD.
local function while_d(me: Object)
    if not frozen(me) and owner_action(me) ~= 0xD then
        me:set_lifecycle("destroy")
    end
    ride(me)
end

-- `sub_80E3F2A`: riding on the owner while it is in action 0xA, shown
-- with it.
local function while_a(me: Object)
    if not frozen(me) and owner_action(me) ~= 0xA then
        me:set_lifecycle("destroy")
    end
    ride(me)
    me.visible = (me.related1 :: Object).visible
end

-- `sub_80E3F6C`: 30 ticks at most, while the owner is in action 0xB.
local function timed(me: Object)
    me.hp = 1
    if me.action == 0 then
        me.action = 4
        me.timer = 0x1E
    end
    if battle.dimmed() then
        return
    end
    me.hp = 0
    if owner_action(me) == 0xB then
        local left = me.timer - 1
        me.timer = left
        if left > 0 then
            return
        end
    end
    me:set_lifecycle("destroy")
end

local function none(_me: Object) end

local VARIANTS = { [0] = none, none, none, slide, flash, while_b, while_d, while_a, timed }

-- `sub_80E3DE0`: the variant, then shown as the owner is.
local function tick(me: Object)
    local f = VARIANTS[me:param(1)]
    if not f then
        error(string.format("follow effect %d reads past its table (off_80E3E10)", me:param(1)))
    end
    f(me)
    me.visible = (me.related1 :: Object).visible
end

-- `sub_80E3D90`: its look, then its first update.
local function init(me: Object)
    local l = LOOKS[me:param(1)]
    if not l then
        error(string.format("follow effect %d reads past byte_80E3D44", me:param(1)))
    end
    local sprite = me.sprite
    sprite:load(l.sprite)
    sprite.shadow = "with_sprite"
    me.anim = l.anim
    me.anim_loaded = l.anim
    sprite:set_animation(l.anim)
    sprite.palette = l.palette + me:param(3)
    sprite:set_flip(me:param(2))
    me.visible = true
    me.hp = 0
    me:set_lifecycle("update")
    tick(me)
end

-- `sub_80E3D68`: its sprite steps unless held (HP bit 0).
function follow_effect.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        me:free()
    end
    if bit32.band(me.hp, 1) == 0 then
        me:update_sprite()
    end
end

return follow_effect
