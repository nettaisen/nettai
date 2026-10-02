--!strict
-- LifeSync's marker (the original's effect object #0x60, `sub_80E76F8`;
-- docs/engine/dimming-chip-effects.md §14): a cursor over the panel two
-- ahead of the user, then over the enemy there, for 81 ticks. Its sprite
-- steps once, when it appears. Only drawn; LifeSync's controller places
-- it outside link battles only.

local marker = {}

export type State = {
    -- Its lifetime (Param1, 0x50).
    ticks: number,
}

local SPRITE = asset.sprite("hit-sparks")
local ANIM, PALETTE = 0xB, 2

-- `sub_80E7756`: the lifetime on the first tick; freed at once when it
-- runs out.
local function tick(me: Object)
    if me.phase_init == 0 then
        me.timer = (me.state :: State).ticks
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = t
    if t < 0 then
        me:free()
    end
end

marker.kind = define.kind {
    id = "lifesync/marker",
    pool = "effect",
    state = { ticks = "u8" },
    -- `sub_80E76F8`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            -- sub_80E7718: animation 0xB, stepped once, palette 2; on at
            -- once.
            local sprite = me.sprite
            sprite:load(SPRITE)
            sprite.shadow = "with_sprite"
            me.anim, me.anim_loaded = ANIM, ANIM
            sprite:set_animation(ANIM)
            sprite:step()
            me.visible = true
            sprite.palette = PALETTE
            me:set_lifecycle("update")
            tick(me)
        elseif l == "update" then
            tick(me)
        else
            me:free()
        end
    end,
}

-- `sub_80E7774`: a marker at `pos` for `ticks` ticks.
function marker.spawn(pos: Vec3, ticks: number): Object?
    local o = battle.spawn(marker.kind, pos)
    if o then
        (o.state :: State).ticks = ticks
    end
    return o
end

return marker
