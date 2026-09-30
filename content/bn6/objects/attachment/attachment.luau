--!strict
-- Attachments (the original's actor object #5, `sub_80B8CD8`): a sprite a
-- navi holds during an attack, such as a bomb before it's thrown or the
-- buster's arm. Visual only, but it takes an actor slot and a place in the
-- update order. It follows its owner's position and visibility, and ends
-- itself once the owner clears the slot it was stored in. See
-- objects-and-player.md §A.2.
--
-- What it shows is a look (`attachment.look { ... }`), a record its user
-- defines; how it animates is the user's too (`attachment.attach(owner,
-- look, slot, { anim = ... })`). The numeric API (`attachment.spawn`,
-- `spawn_with`, by the pack data's attachment rows) stays until its users
-- convert.

local slot = require("../../lib/slot")

local attachment = {}

export type State = {
    slot: Slot,
    look: AttachmentLook,
    anim: number,
    while_dimmed: boolean,
    palette_add: number,
    -- Offset from the owner's position (its attach point), 16.16.
    offset_x: number,
    offset_z: number,
    -- Pixels raised by (subtracted from y and z).
    lift: number,
}

-- An attachment's look (see AttachmentLook in types.d.luau).
function attachment.look(spec: AttachmentLook): AttachmentLook
    return define.record("attachment-look", spec)
end

-- How it animates: its animation, whether it animates while dimmed too,
-- and what is added to its palette (drawn only).
export type Options = { anim: number?, while_dimmed: boolean?, palette_add: number? }

-- `sub_80B8DA6`: follow the owner's position and visibility; animate,
-- except while paused (or dimmed, unless it animates then too); end once
-- the slot is cleared.
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
    if not s.while_dimmed and battle.dimmed() then
        return
    end
    if battle.paused() then
        return
    end
    me:update_sprite()
end

-- `sub_80B8CF8`.
local function init(me: Object)
    local s = me.state :: State
    local k = s.look
    local owner = me.related1 :: Object
    local sprite = me.sprite
    sprite:load(k.sprite)
    sprite.shadow = "with_sprite"
    sprite.palette = k.palette + s.palette_add
    me.no_sprite_update = false
    -- The Cross forms' NameIDs raise it by 8 pixels.
    local name = owner.name_id
    local cross = name >= 0x1AC and name <= 0x1B5 and battle.navi(me.alliance).form ~= 0
    s.lift = if cross then -8 else k.lift or 0
    local anim = s.anim
    me.anim = anim
    me.anim_loaded = anim
    me.visible = true
    sprite:set_animation(anim)
    local x, z = 0, 0
    if k.by_owner then
        -- It sits at a fixed offset by its owner's NameID, not at an attach
        -- point, and not mirrored: none on base MegaMan, (11, -7) on NameID
        -- 0x1B6, (0, 13) on anyone else.
        local name = owner.name_id
        if name == 0x1B6 then
            x, z = 0xB, -7
        elseif name ~= 0x1A0 then
            x, z = 0, 0xD
        end
    elseif k.attach_point then
        x, z = owner:attach_point(k.attach_point)
    end
    s.offset_x = x * 0x10000
    s.offset_z = z * 0x10000
    me:set_lifecycle("update")
    follow(me)
end

attachment.kind = define.kind {
    id = "attachment",
    pool = "actor",
    state = {
        slot = slot.TYPE,
        look = "record:attachment-look",
        anim = "u8",
        while_dimmed = "bool",
        palette_add = "u8",
        offset_x = "i32",
        offset_z = "i32",
        lift = "i8",
    },
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            follow(me)
        else
            me:free()
        end
    end,
}

-- `sub_80B8E30`: attach `look` to `owner`, stored in `which` (which gets
-- nil if the pool is full). It takes the owner's panel and side; its init
-- places it.
function attachment.attach(owner: Object, look: AttachmentLook, which: Slot, options: Options?): Object?
    local o = battle.spawn(attachment.kind, Vec3.zero)
    if o then
        o.related1 = owner
        o.panel_x, o.panel_y = owner.panel_x, owner.panel_y
        o.alliance, o.flip = owner.alliance, owner.flip
        o.run_while_paused = true
        o.run_while_dimmed = true
        local s = o.state :: State
        s.slot = which
        s.look = look
        local opt: Options = options or {}
        s.anim = opt.anim or 0
        s.while_dimmed = opt.while_dimmed or false
        s.palette_add = opt.palette_add or 0
    end
    slot.fill(owner, which, o)
    return o
end

-- The numeric API, until its users convert: the pack data's attachment
-- rows (data.objects.attachments) as looks, by row (row 0xF sits by its
-- owner's NameID).
local ROWS: { [number]: AttachmentLook } = {}
do
    local ids: { number } = {}
    for id in data.objects.attachments do
        table.insert(ids, id)
    end
    table.sort(ids)
    for _, id in ids do
        local a = data.objects.attachments[id]
        ROWS[id] = attachment.look {
            sprite = a.sprite,
            palette = a.palette,
            lift = a.lift,
            attach_point = a.attach_point,
            by_owner = id == 0xF,
        }
    end
end

-- What an attachment is spawned with by row (its four parameters).
export type Params = {
    -- Param1: which row (data.objects.attachments).
    kind: number,
    -- Param2: its animation.
    anim: number?,
    -- Param3: it animates while dimmed too.
    while_dimmed: boolean?,
    -- Param4: added to its palette (drawn only).
    palette_add: number?,
}

-- Deprecated: an attachment by row; use `attach` with a look.
function attachment.spawn(owner: Object, p: Params, which: Slot): Object?
    local look = ROWS[p.kind]
    if not look then
        error(string.format("attachment kind %#x is not in the content", p.kind))
    end
    return attachment.attach(owner, look, which, { anim = p.anim, while_dimmed = p.while_dimmed, palette_add = p.palette_add })
end

-- Deprecated: the same with the four parameters as bytes.
function attachment.spawn_with(owner: Object, params: { number }, which: Slot): Object?
    return attachment.spawn(
        owner,
        { kind = params[1], anim = params[2], while_dimmed = params[3] ~= 0, palette_add = params[4] },
        which
    )
end

return attachment
