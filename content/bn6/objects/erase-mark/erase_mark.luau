--!strict
-- A mark on a panel EraseMan aims at (effect object #0x62, `sub_80E78BC`):
-- it shows for Param1 ticks, dimmed or not. Purely visual, but it holds an
-- effect slot. See docs/engine/chips.md §3.6.10.

local erase_mark = {}

local SPRITE = "10-50"

-- `sub_80E7942`: a mark on (x, y) for `ticks` ticks, on `owner`'s side.
-- (Its Z is the 1 its spawner leaves in a register, of which the init
-- keeps the fraction.)
function erase_mark.spawn(owner: Object, x: number, y: number, ticks: number): Object?
    local o = battle.spawn_kind("erase-mark", Vec3.new(0, 0, 1), { ticks })
    if o then
        o.panel_x, o.panel_y = x, y
        o.alliance, o.flip = owner.alliance, owner.flip
    end
    return o
end

-- `sub_80E78E0`: on its panel, at the fraction of its Z.
local function init(me: Object)
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, bit32.band(p.z, 0xFFFF))
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim = 0
    me.anim_loaded = 0
    sprite:set_animation(0)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.timer = me:param(1)
    me:set_lifecycle("update")
end

-- `sub_80E792A`: gone once the battle is over or its time is up.
local function tick(me: Object)
    if battle.over() then
        me:free()
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left <= 0 then
        me:free()
    end
end

-- `sub_80E78BC`.
function erase_mark.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        me:free()
    end
    me:update_sprite_while_dimmed()
end

return erase_mark
