--!strict
-- GridMan's grid (EXE5's effect object #0x58, 0x080E6D3C; EXE5's own): his sprite's animation 13 on the far end of
-- his dash (the middle row of the column he strikes from), 32 pixels up the field and down into it, facing back;
-- it stays until he goes (`grid.remove`) or the battle ends.
--
-- Its first parameter keeps it: GridMan's is 1. With 0 it would go on its first update unless its owner were in
-- action 0xB, which GridMan never is (0x080E6DC0).

local grid = {}

export type State = {
    -- Its first parameter (GridMan's: 1).
    stays: boolean,
}

local SPRITE = asset.sprite("gridman")

-- 0x080E6D60: on its panel's center, 32 pixels up the field and 32 down, drawn with its sprite.
local function init(me: Object)
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, int.i32(p.y - 0x20_0000), int.i32(-0x20_0000))
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim, me.anim_loaded = 13, 13
    sprite:set_animation(13)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_lifecycle("update")
end

-- 0x080E6DB0: gone with the battle (or, without its parameter, unless its owner is in action 0xB).
local function tick(me: Object)
    if battle.over() then
        me:free()
        return
    end
    if (me.state :: State).stays then
        return
    end
    local owner = me.related1
    if not (owner and owner.action == 0xB) then
        me:free()
    end
end

grid.kind = define.kind {
    id = "gridman/grid",
    pool = "effect",
    state = { stays = "bool" },
    -- 0x080E6D3C.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:free()
        end
        me:update_sprite_while_paused()
    end,
}

-- 0x080E6DD0: the grid of `owner` on (x, y), facing back from him.
function grid.spawn(owner: Object, x: number, y: number): Object?
    local o = battle.spawn(grid.kind, Vec3.zero)
    if o then
        o.panel_x, o.panel_y = x, y
        o.related1 = owner
        o.alliance, o.flip = owner.alliance, bit32.bxor(owner.flip, 1)
        ;(o.state :: State).stays = true
    end
    return o
end

-- 0x080E6DF4: it goes (at its next update).
function grid.remove(me: Object)
    me:set_lifecycle("destroy")
end

return grid
