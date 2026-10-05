--!strict
-- LifeSync's marker, as EXE5 and EXE6 share it (EXE6's effect object #0x60, `sub_80E76F8`; EXE5's #0x60, the same code
-- but for its sprite; docs/engine/dimming-chip-effects.md §14): a cursor over the panel two ahead of the user, then
-- over the enemy there, for 81 ticks. Its sprite steps once, when it appears. Only drawn.
--
-- Each game makes its kind with `marker.make_kind(id, sprite)` (EXE6's chips/lifesync, EXE5's chips/lifesync).

local marker = {}

export type State = {
    -- Its lifetime (Param1, 0x50).
    ticks: number,
}

local ANIM, PALETTE = 0xB, 2

-- `sub_80E7756`: the lifetime on the first tick; freed at once when it runs out.
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

-- A game's marker kind `id`, in `sprite` (its hit sparks).
function marker.make_kind(id: string, sprite: SpriteAsset): Kind
    return define.kind {
        id = id,
        pool = "effect",
        state = { ticks = "u8" },
        -- `sub_80E76F8`.
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                -- sub_80E7718: animation 0xB, stepped once, palette 2; on at once.
                local s = me.sprite
                s:load(sprite)
                s.shadow = "with_sprite"
                me.anim, me.anim_loaded = ANIM, ANIM
                s:set_animation(ANIM)
                s:step()
                me.visible = true
                s.palette = PALETTE
                me:set_lifecycle("update")
                tick(me)
            elseif l == "update" then
                tick(me)
            else
                me:free()
            end
        end,
    }
end

-- `sub_80E7774`: a marker of `kind` at `pos` for `ticks` ticks.
function marker.spawn(kind: Kind, pos: Vec3, ticks: number): Object?
    local o = battle.spawn(kind, pos)
    if o then
        (o.state :: State).ticks = ticks
    end
    return o
end

return marker
