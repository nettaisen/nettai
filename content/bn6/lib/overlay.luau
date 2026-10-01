--!strict
-- An overlay (the original's actor object #0x54, `sub_80C3EE0`;
-- docs/engine/dimming-chips.md §7.4): a sprite drawn over its owner, a
-- pixel below or above it (and optionally higher), that follows the
-- owner's position, visibility and look every tick and animates unless
-- the owner is held. AirRaid's plane's jet flame is one; LilBoiler's
-- and other objects' are too.

local overlay = {}

-- What an overlay shows and how it follows (`sub_80C4038`'s arguments and
-- spawn parameters).
export type Spec = {
    sprite: SpriteAsset,
    anim: number,
    -- Its palette (Param4); nil, the owner's, every tick (Param3 0).
    palette: number?,
    -- Added to the owner's Z (16.16, EV+0xC).
    z: number?,
    -- A pixel above the owner (EV+8), else a pixel below.
    up: boolean?,
    -- Its own flip, not the owner's (EV+0).
    keeps_flip: boolean?,
    -- Raised by 24 pixels a row above the bottom one (EV+0x10).
    by_row: boolean?,
    -- Still while the owner has no HP (EV+0x14).
    still_when_owner_down: boolean?,
    -- Its own visibility and colour shader (EV+0x18).
    own_look: boolean?,
}

export type State = {
    sprite: SpriteAsset,
    anim: number,
    own_palette: boolean,
    palette: number,
    z: number,
    up: boolean,
    keeps_flip: boolean,
    by_row: boolean,
    still_when_owner_down: boolean,
    own_look: boolean,
}

-- The owner is held (bubbled, dragged, frozen, paralyzed or flinching:
-- its status word has 0x80110C00).
local function held(owner: Object): boolean
    return owner:status("bubbled")
        or owner:status("drag")
        or owner:status("frozen")
        or owner:status("paralyzed")
        or owner:status("flinching")
end

-- `sub_80C409C`: its panel from its position, raised by 24 pixels a row
-- above the bottom one, and a pixel more.
local function raise_by_row(me: Object)
    me:set_panel_from_coordinates()
    local d = (3 - me.panel_y) * 0x18_0000
    local p = me.pos
    me.pos = Vec3.new(p.x, int.i32(p.y + d + 0x1_0000), int.i32(p.z + d + 0x1_0000))
end

-- `sub_80C3F52`.
local function follow(me: Object)
    local s = me.state :: State
    local owner = me.related1 :: Object
    local p = owner.pos
    local dz = if s.up then -0x1_0000 else 0x1_0000
    me.pos = Vec3.new(p.x, int.i32(p.y + dz), int.i32(p.z + s.z + dz))
    local look, worn = me.sprite, owner.sprite
    if not s.own_look then
        me.visible = owner.visible
    end
    if not s.own_palette then
        look.palette = worn.palette
    end
    if not s.own_look then
        look.color_shader = worn.color_shader
    end
    look.white = worn.white
    look.mosaic = worn.mosaic
    if not s.keeps_flip then
        me.flip = owner.flip
        look:set_flip(bit32.bxor(me.alliance, me.flip))
    end
    look.alpha = worn.alpha
    if s.by_row then
        raise_by_row(me)
    end
    if battle.dimmed() or battle.paused() or held(owner) then
        return
    end
    if s.still_when_owner_down and owner.hp == 0 then
        return
    end
    me:update_sprite()
end

-- `sub_80C3F00`: its look, then it follows.
local function init(me: Object)
    local s = me.state :: State
    me:set_panel_from_coordinates()
    local sprite = me.sprite
    sprite:load(s.sprite)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim, me.anim_loaded = s.anim, s.anim
    sprite:set_animation(s.anim)
    sprite.palette = if s.own_palette then s.palette else (me.related1 :: Object).sprite.palette
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_lifecycle("update")
    follow(me)
end

overlay.kind = define.kind {
    id = "overlay",
    pool = "actor",
    state = {
        sprite = "sprite",
        anim = "u8",
        own_palette = "bool",
        palette = "u8",
        z = "i32",
        up = "bool",
        keeps_flip = "bool",
        by_row = "bool",
        still_when_owner_down = "bool",
        own_look = "bool",
    },
    -- `sub_80C3EE0`.
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

-- `sub_80C4038`: an overlay on `owner` (at its position, side and facing).
-- It runs while dimmed and paused.
function overlay.spawn(owner: Object, spec: Spec): Object?
    local o = battle.spawn(overlay.kind, owner.pos)
    if not o then
        return nil
    end
    o.alliance, o.flip = owner.alliance, owner.flip
    o.related1 = owner
    o.run_while_dimmed = true
    o.run_while_paused = true
    local s = o.state :: State
    s.sprite, s.anim = spec.sprite, spec.anim
    s.own_palette, s.palette = spec.palette ~= nil, spec.palette or 0
    s.z = spec.z or 0
    s.up = spec.up == true
    s.keeps_flip = spec.keeps_flip == true
    s.by_row = spec.by_row == true
    s.still_when_owner_down = spec.still_when_owner_down == true
    s.own_look = spec.own_look == true
    return o
end

-- `sub_80C4072`: it goes (at its next update).
function overlay.stop(o: Object)
    o:set_lifecycle("destroy")
end

return overlay
