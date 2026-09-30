--!strict
-- A navi's effect (effect object #0x31, `sub_80E3D68`): one of nine looks
-- by Param1 (`byte_80E3D44`), each with its own routine (`off_80E3E10`);
-- Param2 is its flip, Param3 added to its palette. It shows while its
-- owner (related1) does. Its HP is a flag: 1 skips its sprite's step that
-- tick (it stands still while dimmed). ElecMan's DElecSwd spawns look 7,
-- which rides on him while he is in action 0xA. [unverified: no lab
-- scenario reaches it; docs/engine/standard-chips.md]

local navi_effect = {}

-- By Param1: the sprite, its animation and palette.
local LOOKS = {
    [0] = { sprite = "10-00", anim = 0, palette = 0 },
    { sprite = "10-00", anim = 0, palette = 0 },
    { sprite = "10-00", anim = 0, palette = 0 },
    { sprite = "0c-00", anim = 2, palette = 0 },
    { sprite = "10-0e", anim = 0, palette = 0 },
    { sprite = "10-11", anim = 0, palette = 0 },
    { sprite = "0c-38", anim = 0x0E, palette = 0 },
    { sprite = "0c-38", anim = 0x0F, palette = 0 },
    { sprite = "10-4a", anim = 1, palette = 0 },
}
-- Look 3's shudder, in whole pixels (`dword_80E3EB0`), backward for flip 0.
local SHUDDER = { [0] = 2, 3, 2, 0 }

-- `sub_80E3FB4`: look `params[1]` at `pos`, for `owner`.
function navi_effect.spawn(owner: Object, pos: Vec3, params: { number }): Object?
    local o = battle.spawn_kind("navi-effect", pos, params)
    if o then
        o.related1 = owner
    end
    return o
end

-- `sub_80E3FC4`: end it (its next update frees it).
function navi_effect.stop(o: Object)
    o:set_lifecycle("destroy")
end

-- Its owner's position, as the game copies X, Y and Z.
local function ride(me: Object)
    me.pos = (me.related1 :: Object).pos
end

-- `sub_80E3E38`, look 3: beside its owner (16 pixels ahead), shuddering
-- for four ticks; once the owner's related1 is gone, animation 3 plays
-- out and it ends.
local function beside(me: Object)
    -- The action it starts from is left in r0 as the entry's sound call
    -- returns it (the sound id), so the entry tick goes on as action 2.
    local r0 = me.action
    if r0 == 0 then
        me.action = 1
        me.timer2 = 0
        battle.play_sound(0xA0)
        r0 = 0xA0
    end
    if r0 ~= 1 then
        if bit32.btest(me.sprite.frame_flags, 0x80) then
            me:set_lifecycle("destroy")
        end
        return
    end
    local owner = me.related1
    local flipped = me:param(2) ~= 0
    if owner then
        local ahead = if flipped then -0x10 else 0x10
        local p = owner.pos
        me.pos = Vec3.new(p.x + ahead * 0x10000, p.y, me.pos.z)
    end
    local step = me.timer2
    if step ~= 4 then
        local d = SHUDDER[step]
        if d == nil then
            error(string.format("navi effect shudder step %d reads past dword_80E3EB0", step))
        end
        if not flipped then
            d = -d
        end
        -- The whole pixels of X (the halfword above its fraction).
        local x = me.pos.x
        local whole = bit32.band(bit32.rshift(x, 16) + d, 0xFFFF)
        me.pos = Vec3.new(int.i32(bit32.bor(bit32.band(x, 0xFFFF), bit32.lshift(whole, 16))), me.pos.y, me.pos.z)
        me.timer2 = step + 1
    end
    if owner == nil then
        error("navi effect look 3 without an owner reads its related1 through a null pointer")
    end
    if owner.related1 == nil then
        me.anim = 3
        me.action = 2
    end
end

-- `sub_80E3EB4`, look 4: 60 ticks, animation 1 from the sixth.
local function flash(me: Object)
    if me.action == 0 then
        me.action = 4
        me.timer = 0x3C
    end
    if me.timer < 0x37 then
        me.anim = 1
    end
    local t = me.timer - 1
    me.timer = t
    if t <= 0 then
        me:set_lifecycle("destroy")
    end
end

-- Looks 5 to 7: stand still while dimmed; end once the owner leaves
-- `action`.
local function while_owner_in(me: Object, action: number): boolean
    me.hp = 1
    if battle.dimmed() then
        return false
    end
    me.hp = 0
    if (me.related1 :: Object).action ~= action then
        me:set_lifecycle("destroy")
    end
    return true
end

-- `sub_80E3F6C`, look 8: 30 ticks while its owner is in action 0xB.
local function countdown_in_b(me: Object)
    me.hp = 1
    if me.action == 0 then
        me.action = 4
        me.timer = 0x1E
    end
    if battle.dimmed() then
        return
    end
    me.hp = 0
    if (me.related1 :: Object).action == 0xB then
        local t = me.timer - 1
        me.timer = t
        if t > 0 then
            return
        end
    end
    me:set_lifecycle("destroy")
end

-- `sub_80E3DE0`: the look's routine, then show as the owner shows.
local function tick(me: Object)
    local look = me:param(1)
    if look <= 2 then
        -- `sub_80E3E34`: nothing.
    elseif look == 3 then
        beside(me)
    elseif look == 4 then
        flash(me)
    elseif look == 5 then
        -- `sub_80E3EDC`
        while_owner_in(me, 0xB)
    elseif look == 6 then
        -- `sub_80E3EFC`
        while_owner_in(me, 0xD)
        ride(me)
    elseif look == 7 then
        -- `sub_80E3F2A`
        while_owner_in(me, 0xA)
        ride(me)
        me.visible = (me.related1 :: Object).visible
    elseif look == 8 then
        countdown_in_b(me)
    else
        error(string.format("navi effect look %d reads past off_80E3E10", look))
    end
    me.visible = (me.related1 :: Object).visible
end

-- `sub_80E3D90`.
local function init(me: Object)
    local look = LOOKS[me:param(1)]
    if look == nil then
        error(string.format("navi effect look %d reads past byte_80E3D44", me:param(1)))
    end
    local sprite = me.sprite
    sprite:load(look.sprite)
    sprite:set_animation(0)
    sprite.shadow = "with_sprite"
    me.anim = look.anim
    me.anim_loaded = look.anim
    sprite:set_animation(look.anim)
    sprite.palette = look.palette + me:param(3)
    sprite:set_flip(me:param(2))
    me.visible = true
    me.hp = 0
    me:set_lifecycle("update")
    tick(me)
end

-- `sub_80E3D68`.
function navi_effect.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        me:free()
        return
    end
    if bit32.band(me.hp, 1) == 0 then
        me:update_sprite()
    end
end

return navi_effect
