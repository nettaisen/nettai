--!strict
-- A water mark of SpoutMan's geyser (effect object #0x2E, `sub_80E39A0`):
-- 2 pixels up and down from its panel's centre it shows for Param2 ticks,
-- the geyser's own column (Param1 1) or the trail to it (Param1 0). With
-- Param3 1 (SpoutMan's) it counts down and steps its sprite while dimmed;
-- without, it waits a dimming out, and goes as soon as its owner isn't in
-- action 0xB. Purely visual, but it holds an effect slot. See
-- docs/engine/chips.md §3.6.11.

local spout_mark = {}

local SPRITE = "10-20"

-- The animation by Param1 (`word_80E3A30`).
local ANIMS = { [0] = 1, 2 }

-- The owner's action the marks of a navi AI's geyser last through.
local OWNER_ACTION = 0xB

-- `sub_80E3A92`: a mark on (x, y) for `owner` (the geyser's navi), on
-- `spawner`'s side; `params` {Param1, ticks, Param3}. `z` is the
-- spawner's register that becomes its Z, whose fraction it keeps.
function spout_mark.spawn(spawner: Object, owner: Object?, x: number, y: number, params: { number }, z: number): Object?
    local o = battle.spawn_kind("spout-mark", Vec3.new(y, 0, z), params)
    if o then
        o.panel_x, o.panel_y = x, y
        o.related1 = owner
        o.alliance, o.flip = spawner.alliance, spawner.flip
        o.run_while_dimmed = true
    end
    return o
end

-- `sub_80E3A74`: count Param2 ticks down, the first on the tick it starts.
local function show(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.timer = me:param(2)
    end
    local left = me.timer - 1
    me.timer = left
    if left <= 0 then
        me:set_lifecycle("destroy")
    end
end

-- `sub_80E3A32`: gone once the battle is over (or, not SpoutMan's, its
-- owner is out of action 0xB); outside a dimming unless it is SpoutMan's.
local function tick(me: Object)
    local own = me:param(3) == 1
    if battle.over() or (not own and (me.related1 :: Object).action ~= OWNER_ACTION) then
        me:set_lifecycle("destroy")
        return
    end
    if not own and battle.dimmed() then
        return
    end
    if me.action ~= 0 then
        error(string.format("SpoutMan's mark action %#x reads past its table (off_80E3A70)", me.action))
    end
    show(me)
end

-- `sub_80E39D0`: on its panel, 2 pixels up and down (whole-part halfword
-- stores), then its first update at once.
local function init(me: Object)
    me:set_coordinates_from_panel()
    local p = me.pos
    local z = bit32.bor(bit32.band(p.z, 0xFFFF), 2 * 0x10000)
    me.pos = Vec3.new(p.x, p.y + 2 * 0x10000, z)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    local anim = ANIMS[me:param(1)]
    if not anim then
        error(string.format("SpoutMan's mark %d reads past word_80E3A30", me:param(1)))
    end
    me.anim = anim
    me.anim_loaded = anim
    sprite:set_animation(anim)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_lifecycle("update")
    tick(me)
end

-- `sub_80E39A0`.
function spout_mark.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        me:free()
    end
    if me:param(3) == 1 then
        me:update_sprite_while_dimmed()
    else
        me:update_sprite_while_paused()
    end
end

return spout_mark
