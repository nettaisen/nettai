--!strict
-- NumberSoul's layer (EXE5's actor object 0x54, 0x080C32C4; EXE5's own, where EXE6's 0x54 is LilBoiler's layer): a second
-- sprite on its owner, by its row (0x080C32C0): row 0 NumberMan's face (10-06, on NumberSoul's image), row 1 MegaMan's
-- in NumberSoul (10-01, his start hook's, 0x0800F07C). It follows its owner's position, visibility, palette, color
-- shader, white flash, alpha and facing; while its owner isn't in his animation 0 it sits 255 pixels up (its height's
-- upper half 0xFF, out of sight), and in it at his height's fraction. It steps unless the battle is paused or dimmed.
-- Its owner takes it off (0x080C33EA: it goes at its next update).

local layer = {}

export type State = {
    -- Param1: its row.
    row: number,
}

-- Its sprites by row (0x080C32C0).
local SPRITES: { [number]: SpriteAsset } = {
    [0] = asset.sprite("numberman-face"),
    [1] = asset.sprite("numbersoul-face"),
}

-- 0x080C3332: on its owner, wearing his look; out of sight unless he is in his animation 0.
local function follow(me: Object)
    local owner = me.related1 :: Object
    local p = owner.pos
    local z = bit32.band(p.z, 0xFFFF)
    if owner.anim ~= 0 then
        z = bit32.bor(z, 0xFF_0000)
    end
    me.pos = Vec3.new(p.x, p.y, int.i32(z))
    me.visible = owner.visible
    local look, worn = me.sprite, owner.sprite
    look.palette = worn.palette
    look.color_shader = worn.color_shader
    look.white = worn.white
    look.alpha = worn.alpha
    me.flip = owner.flip
    look:set_flip(bit32.bxor(me.alliance, me.flip))
    if not battle.dimmed() and not battle.paused() then
        me:update_sprite()
    end
end

-- 0x080C32E4: its row's sprite (no shadow) at animation 0, shown, in its owner's palette and facing its way.
local function init(me: Object)
    local s = me.state :: State
    me:set_panel_from_coordinates()
    local sprite = me.sprite
    local look = SPRITES[s.row]
    if not look then
        error(string.format("NumberSoul's layer's row %d reads past its sprites (0x080C32C0)", s.row))
    end
    sprite:load(look)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    sprite.palette = (me.related1 :: Object).sprite.palette
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_lifecycle("update")
end

layer.kind = define.kind {
    id = "numbersoul/layer",
    pool = "actor",
    state = { row = "u8" },
    -- 0x080C32C4
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

-- 0x080C33BE: a layer of `row` on `owner`, at his position, of his side (its facing his from its first update),
-- running while paused and dimmed (kept in his second related slot by its callers).
function layer.spawn(owner: Object, row: number): Object?
    local o = battle.spawn(layer.kind, owner.pos)
    if o then
        (o.state :: State).row = row
        o.alliance = owner.alliance
        o.related1 = owner
        o.run_while_paused = true
        o.run_while_dimmed = true
    end
    return o
end

-- 0x080C33EA: take `o` off (it goes at its next update).
function layer.take_off(o: Object?)
    if o then
        o:set_lifecycle("destroy")
    end
end

return layer
