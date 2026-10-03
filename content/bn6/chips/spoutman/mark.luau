--!strict
-- A water mark of SpoutMan's geyser (the original's effect object #0x2E,
-- `sub_80E39A0`): 2 pixels up and down from its panel's center it shows
-- for its ticks, the geyser's own column or the trail to it. The navi
-- chip's (the original's Param3 1) counts down and steps its sprite while
-- dimmed; another waits a dimming out, and goes as soon as its owner isn't
-- in action 0xB. Purely visual, but it holds an effect slot. What it is
-- spawned with is a `GeyserMark` (types.d.luau). See docs/engine/chips.md
-- §3.6.11.

local mark = {}

export type State = {
    -- It marks the geyser's column (Param1 1), not the trail to it.
    column: boolean,
    -- How long it shows (Param2).
    ticks: number,
    -- It counts down, and animates, while the battle is dimmed (Param3).
    while_dimmed: boolean,
}

local SPRITE = asset.sprite("bn6:spout-geyser")

-- Its animation (`word_80E3A30`): the trail's, the column's.
local ANIM_TRAIL, ANIM_COLUMN = 1, 2

-- The owner's action the marks of a navi AI's geyser last through.
local OWNER_ACTION = 0xB

-- `sub_80E3A74`: count its ticks down, the first on the tick it starts.
local function show(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.timer = (me.state :: State).ticks
    end
    local left = me.timer - 1
    me.timer = left
    if left <= 0 then
        me:set_lifecycle("destroy")
    end
end

-- `sub_80E3A32`: gone once the battle is over (or, not the navi chip's,
-- its owner is out of action 0xB); outside a dimming unless it is the navi
-- chip's.
local function tick(me: Object)
    local own = (me.state :: State).while_dimmed
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
    local anim = if (me.state :: State).column then ANIM_COLUMN else ANIM_TRAIL
    me.anim = anim
    me.anim_loaded = anim
    sprite:set_animation(anim)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_lifecycle("update")
    tick(me)
end

mark.kind = define.kind {
    id = "bn6:spoutman/mark",
    pool = "effect",
    state = { column = "bool", ticks = "u8", while_dimmed = "bool" },
    -- `sub_80E39A0`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:free()
        end
        if (me.state :: State).while_dimmed then
            me:update_sprite_while_dimmed()
        else
            me:update_sprite_while_paused()
        end
    end,
}

-- `sub_80E3A92`: a mark on (x, y) for `owner` (the geyser's navi), on
-- `spawner`'s side, and what it is. `z` is the spawner's register that
-- becomes its Z, whose fraction it keeps.
function mark.spawn(spawner: Object, owner: Object?, x: number, y: number, what: GeyserMark, z: number): Object?
    local o = battle.spawn(mark.kind, Vec3.new(y, 0, z))
    if o then
        local s = o.state :: State
        s.column = what.column == true
        s.ticks = what.ticks
        s.while_dimmed = what.while_dimmed == true
        o.panel_x, o.panel_y = x, y
        o.related1 = owner
        o.alliance, o.flip = spawner.alliance, spawner.flip
        o.run_while_dimmed = true
    end
    return o
end

return mark
