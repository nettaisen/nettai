--!strict
-- A layer (the original's actor object #0x54, `sub_80C3EE0`): a second
-- sprite drawn with its owner, a pixel in front of it (or behind), that
-- follows it every tick: its position (and a Z offset), its visibility,
-- palette, colour shader, flip and blending. The erupting LilBoiler shows
-- its steam with one, AirRaid's plane its jet flame (chips/airraid); the
-- viruses use it too. See docs/engine/shot-chips.md §14.1.

local layer = {}

export type State = {
    -- Its look (Param1 and Param2) and, when `own_palette` (Param3), its
    -- palette (Param4) rather than its owner's.
    sprite: SpriteAsset,
    own_palette: boolean,
    palette: number,
    -- Its animation (ExtraVars+4), its height over its owner (16.16,
    -- ExtraVars+0xC), and drawn a pixel behind its owner rather than in
    -- front (ExtraVars+8).
    anim: number,
    z_offset: number,
    behind: boolean,
    -- Its own flip rather than its owner's (ExtraVars); a row offset
    -- (ExtraVars+0x10), its sprite held while its owner has no HP
    -- (+0x14), its own visibility and colour shader (+0x18): the viruses'
    -- settings (`sub_80C407C`, `sub_80C4086`, `sub_80C4090`).
    own_flip: boolean,
    row_offset: boolean,
    holds_without_hp: boolean,
    own_look: boolean,
}

-- What a layer is spawned with (`sub_80C4038`'s registers and parameters).
export type LayerSpec = {
    sprite: SpriteAsset,
    anim: number,
    z_offset: number,
    behind: boolean,
    own_flip: boolean,
    -- Its own palette, else its owner's.
    palette: number?,
}

local PIXEL = 0x1_0000
-- The owner's statuses that hold its sprite: bubbled, drag, frozen,
-- paralyzed, flinching (0x80110C00).
local HOLDING: { StatusFlag } = { "frozen", "drag", "paralyzed", "flinching" }

local function holds(owner: Object): boolean
    for _, f in HOLDING do
        if owner:status(f) then
            return true
        end
    end
    return owner:status("bubbled" :: StatusFlag)
end

-- `sub_80C409C`: a row offset: its panel from its position; Y and Z down
-- the field by (3 - row) panels' 24 pixels, and a pixel more (on their
-- whole halves).
local function row_offset(me: Object)
    me:set_panel_from_coordinates()
    local d = (3 - me.panel_y) * 24 * PIXEL
    local p = me.pos
    me.pos = Vec3.new(int.i32(p.x), int.i32(p.y + d + PIXEL), int.i32(p.z + d + PIXEL))
end

-- `sub_80C3F52`: over its owner, as its owner looks; its sprite steps
-- unless the battle is dimmed or paused, its owner is held, or (with
-- `holds_without_hp`) its owner has no HP.
local function tick(me: Object)
    local s = me.state :: State
    local owner = me.related1 :: Object
    local p = owner.pos
    local dz = if s.behind then -PIXEL else PIXEL
    me.pos = Vec3.new(p.x, p.y + dz, int.i32(p.z + s.z_offset + dz))
    local sprite, look = me.sprite, owner.sprite
    if not s.own_look then
        me:copy_visibility(owner)
    end
    if not s.own_palette then
        sprite.palette = look.palette
    end
    if not s.own_look then
        sprite.color_shader = look.color_shader
    end
    -- The owner's final palette (`sprite_setFinalPalette`: its white
    -- flash) and mosaic (`sub_8002F3E`, `sprite_setMosaicSize`).
    sprite.white = look.white
    sprite.mosaic = look.mosaic
    if not s.own_flip then
        me.flip = owner.flip
        sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    end
    sprite.alpha = look.alpha
    if s.row_offset then
        row_offset(me)
    end
    if battle.dimmed() or battle.paused() or holds(owner) then
        return
    end
    if s.holds_without_hp and owner.hp <= 0 then
        return
    end
    me:update_sprite()
end

-- `sub_80C3F00`: its look, on its owner's panel.
local function init(me: Object)
    local s = me.state :: State
    me:set_panel_from_coordinates()
    local sprite = me.sprite
    sprite:load(s.sprite)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim, me.anim_loaded = s.anim, s.anim
    sprite:set_animation(s.anim)
    local owner = me.related1 :: Object
    sprite.palette = if s.own_palette then s.palette else owner.sprite.palette
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_lifecycle("update")
    tick(me)
end

layer.kind = define.kind {
    id = "bn6:lilbolr/layer",
    pool = "actor",
    state = {
        sprite = "sprite",
        own_palette = "bool",
        palette = "u8",
        anim = "u8",
        z_offset = "i32",
        behind = "bool",
        own_flip = "bool",
        row_offset = "bool",
        holds_without_hp = "bool",
        own_look = "bool",
    },
    -- `sub_80C3EE0`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:free()
        end
    end,
}

-- `sub_80C4038`: a layer over `owner`, at its position, of its side and
-- flip; it runs while paused and dimmed. Nil if the actor pool is full.
function layer.spawn(owner: Object, spec: LayerSpec): Object?
    local o = battle.spawn(layer.kind, owner.pos)
    if o then
        local s = o.state :: State
        s.sprite = spec.sprite
        s.own_palette = spec.palette ~= nil
        s.palette = spec.palette or 0
        s.anim = spec.anim
        s.z_offset = spec.z_offset
        s.behind = spec.behind
        s.own_flip = spec.own_flip
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = owner
        o.run_while_paused = true
        o.run_while_dimmed = true
    end
    return o
end

-- `sub_80C4072`: end it (it frees itself at its next update).
function layer.finish(o: Object?)
    if o then
        o:set_lifecycle("destroy")
    end
end

return layer
