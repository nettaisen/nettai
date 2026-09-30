--!strict
-- Attachments (actor object #5, `sub_80B8CD8`): a sprite a navi holds
-- during an attack, such as GunDelSol's gun. Visual only, but it takes an
-- actor slot and a place in the update order. It follows its owner's
-- position and visibility, and ends itself once the owner clears the slot
-- it was stored in. See objects-and-player.md §A.2.

local data = require("../data/pack")
local slot = require("../lib/slot")

export type State = {
    slot: Slot,
    -- Offset from the owner's position (its attach point), 16.16.
    offset_x: number,
    offset_z: number,
    -- Pixels raised by (subtracted from y and z).
    lift: number,
}

local attachment = {
    name = "attachment",
    pool = "actor",
    index = 5,
    state = { slot = slot.TYPE, offset_x = "i32", offset_z = "i32", lift = "i8" },
}

-- `sub_80B8E30`: attach an object of `kind` (data.attachments) to `owner`,
-- stored in `which` (which gets nil if the pool is full). It takes the
-- owner's panel and side; its init places it.
function attachment.spawn(owner: Object, kind: number, which: Slot): Object?
    local o = battle.spawn("actor", attachment.index, Vec3.zero, { kind, 0, 0, 0 })
    if o then
        o.related1 = owner
        o.panel_x, o.panel_y = owner.panel_x, owner.panel_y
        o.alliance, o.flip = owner.alliance, owner.flip
        o.run_while_paused = true
        o.run_in_time_stop = true
        local s = o.state :: State
        s.slot = which
    end
    slot.fill(owner, which, o)
    return o
end

-- `sub_80B8DA6`: follow the owner's position and visibility; animate,
-- except while paused (or in time stop, unless param 3 says otherwise);
-- end once the slot is cleared.
local function follow(me: Object)
    local owner = me.related1 :: Object
    local s = me.state :: State
    local p = owner.pos
    local lift = s.lift * 0x10000
    me.pos = Vec3.new(p.x + s.offset_x, p.y - lift, p.z + s.offset_z - lift)
    me.visible = owner.visible
    -- It wears the owner's colour shader, white flash, alpha and flip.
    local look, worn = me.sprite, owner.sprite
    look.color_shader = worn.color_shader
    look.white = worn.white
    look.alpha = worn.alpha
    look.hflip = worn.hflip
    look.vflip = worn.vflip
    if not slot.occupied(owner, s.slot) then
        me.visible = false
        me:set_lifecycle("destroy")
        return
    end
    if me:param(3) == 0 and battle.time_stop() then
        return
    end
    if battle.paused() then
        return
    end
    me:update_sprite()
end

-- `sub_80B8CF8`.
local function init(me: Object)
    local kind, anim, palette_add = me:param(1), me:param(2), me:param(4)
    local k = data.attachments[kind]
    local owner = me.related1 :: Object
    local sprite = me.sprite
    sprite:load(k.sprite.category, k.sprite.index)
    sprite.shadow = "with_sprite"
    sprite.palette = k.palette + palette_add
    me.no_sprite_update = false
    -- The Cross forms' NameIDs raise it by 8 pixels.
    local name = owner.name_id
    local cross = name >= 0x1AC and name <= 0x1B5 and battle.navi(me.alliance).form ~= 0
    local s = me.state :: State
    s.lift = if cross then -8 else k.lift
    me.anim = anim
    me.anim_loaded = anim
    me.visible = true
    sprite:set_animation(anim)
    if kind == 0xF then
        error("attachment kind 0xF offsets (sub_80B8CF8) are not implemented yet")
    end
    local x, z = 0, 0
    if k.attach_point then
        x, z = owner:attach_point(k.attach_point)
    end
    s.offset_x = x * 0x10000
    s.offset_z = z * 0x10000
    me:set_lifecycle("update")
    follow(me)
end

function attachment.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        follow(me)
    else
        me:free()
    end
end

return attachment
