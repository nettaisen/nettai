--!strict
-- EXE4's attachments (the actor kind 2 that 0x080CD6E0 spawns: init 0x080CD618, follow 0x080CD666): a
-- sprite a navi holds during an attack, such as the buster's muzzle flash. EXE4's own, not EXE5's and EXE6's
-- (@exelib/attachment): its row (0x080CD524, 4 bytes: the sprite's category and index, the palette, the lift) puts it
-- at its owner's position, raised by the lift, never at an attach point; it follows its owner's visibility, color
-- shader, palette, mosaic and flip; it runs while dimmed (header flag 0x10) but not while paused. What it shows is a
-- look (`attachment.look { sprite, palette, lift }`, a row); `attachment.attach(owner, look, slot, anim)`.

local slot = require("@exelib/slot")

local attachment = {}

type State = {
    slot: Slot,
    look: AttachmentLook,
    anim: number,
    -- Its param's third byte (+6): animates while dimmed too. Every EXE4 attach passes 0 (0x080CD6E0's callers).
    while_dimmed: boolean,
}

-- An attachment's look: a row of 0x080CD524.
function attachment.look(spec: AttachmentLook): AttachmentLook
    return new.record("attachment-look", spec)
end

-- 0x080CD666: at the owner's position less the lift (Y and Z, unsigned); its visibility, color shader, palette,
-- mosaic and flip; gone once the slot is empty; animating unless dimmed.
local function follow(me: Object)
    local owner = me.related1 :: Object
    local s = me.state :: State
    local p = owner.pos
    local lift = (s.look.lift or 0) * 0x10000
    me.pos = Vec3.new(p.x, p.y - lift, p.z - lift)
    me:copy_visibility(owner)
    local look, worn = me.sprite, owner.sprite
    look.color_shader = worn.color_shader
    look.palette = worn.palette
    look.mosaic = worn.mosaic
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
    me:update_sprite()
end

-- 0x080CD618.
local function init(me: Object)
    local s = me.state :: State
    local k = s.look
    local sprite = me.sprite
    sprite:load(k.sprite)
    sprite.shadow = "with_sprite"
    me.anim = s.anim
    me.anim_loaded = s.anim
    me.visible = true
    sprite:set_animation(s.anim)
    sprite.palette = k.palette
    me:set_coordinates_from_panel()
    me:set_lifecycle("update")
    follow(me)
end

attachment.kind = new.kind {
    id = "attachment",
    pool = "actor",
    state = { slot = slot.TYPE, look = "record:attachment-look", anim = "u8", while_dimmed = "bool" },
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

-- 0x080CD6E0: attach `look` to `owner` with animation `anim`, stored in `which` (nil if the actor pool is full). It
-- takes the owner's panel and side.
function attachment.attach(owner: Object, look: AttachmentLook, which: Slot, anim: number): Object?
    local o = battle.spawn(attachment.kind, Vec3.zero)
    if o then
        o.related1 = owner
        o.panel_x, o.panel_y = owner.panel_x, owner.panel_y
        o.alliance = owner.alliance
        o.run_while_dimmed = true
        local s = o.state :: State
        s.slot = which
        s.look = look
        s.anim = anim
        s.while_dimmed = false
    end
    slot.fill(owner, which, o)
    return o
end

-- The buster's muzzle flash (row 0x10: 0c-06), which the buster and the charged shot show (0x080EB396, 0x080ECC96).
attachment.muzzle_flash = attachment.look { sprite = asset.sprite("muzzle-flash"), palette = 0 }

return attachment
