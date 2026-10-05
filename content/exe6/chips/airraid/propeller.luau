--!strict
-- AirRaid's plane's propeller (the original's actor object #3,
-- `sub_80B88D0`; docs/engine/dimming-chips.md §7.3): it sits on the
-- plane's first sprite part, following the plane's position, visibility
-- and look every tick, and spins unless the plane is held.

local propeller = {}

export type State = {
    -- Its own flip, not the owner's (ExtraVars+0).
    keeps_flip: boolean,
}

local SPRITE = asset.sprite("propeller")
-- Where it sits on the owner: on the owner's sprite part 1, and 32 pixels
-- nearer (Y) and 31 higher (Z) than the owner.
local PART = 1
local DY, DZ = 0x20_0000, 0x1F_0000

-- The owner is held (bubbled, dragged, frozen, paralyzed or flinching:
-- its status word has 0x80110C00).
local function held(owner: Object): boolean
    return owner:status("bubbled")
        or owner:status("drag")
        or owner:status("frozen")
        or owner:status("paralyzed")
        or owner:status("flinching")
end

-- `sub_80B8932`.
local function follow(me: Object)
    local dir = me.facing
    local owner = me.related1 :: Object
    local p = owner.pos
    local px, py = owner.sprite:part_offset(PART)
    me.pos = Vec3.new(int.i32(p.x + px * 0x1_0000 * dir), int.i32(p.y + DY + py * 0x1_0000), int.i32(p.z + DZ))
    me:copy_visibility(owner)
    local look, worn = me.sprite, owner.sprite
    look.color_shader = worn.color_shader
    look.white = worn.white
    look.mosaic = worn.mosaic
    if not (me.state :: State).keeps_flip then
        me.flip = owner.flip
        look:set_flip(bit32.bxor(me.alliance, me.flip))
    end
    look.alpha = worn.alpha
    if battle.dimmed() or held(owner) then
        return
    end
    me:update_sprite()
end

-- `sub_80B88F0`: its look.
local function init(me: Object)
    me:set_panel_from_coordinates()
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_lifecycle("update")
end

propeller.kind = define.kind {
    id = "airraid/propeller",
    pool = "actor",
    state = { keeps_flip = "bool" },
    -- `sub_80B88D0`.
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

-- `sub_80B89DC`: a propeller on `owner` (at its position, side and
-- facing). It runs while dimmed and paused.
function propeller.spawn(owner: Object, keeps_flip: boolean): Object?
    local o = battle.spawn(propeller.kind, owner.pos)
    if not o then
        return nil
    end
    (o.state :: State).keeps_flip = keeps_flip
    o.alliance, o.flip = owner.alliance, owner.flip
    o.related1 = owner
    o.run_while_dimmed = true
    o.run_while_paused = true
    return o
end

-- `sub_80B8A0A`: it goes (at its next update).
function propeller.stop(o: Object)
    o:set_lifecycle("destroy")
end

return propeller
