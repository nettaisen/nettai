--!strict
-- LifeSync's marker, as the games share it (EXE6's effect object #0x60, `sub_80E76F8`; EXE5's #0x60, the same code but
-- for its sprite; EXE4's #0x60, 0x080E8C78, but for its palette and its place; docs/engine/dimming-chip-effects.md
-- §14): a cursor over the panel two ahead of the user (EXE5's and EXE6's then over the enemy there) for 81 ticks. Its
-- sprite steps once, when it appears. Only drawn.
--
-- Each game makes its kind with `marker.make_kind(look)` (EXE6's, EXE5's and EXE4's chips/lifesync): its sheet and
-- palette, and EXE4's height over its panel, where its init places it (0x080E8CCC; EXE5's and EXE6's stay where they
-- are spawned).

local marker = {}

export type State = {
    -- Its lifetime (Param1, 0x50).
    ticks: number,
}

local ANIM = 0xB

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

-- A game's marker kind.
function marker.make_kind(look: LifeSyncMarkerLook): Kind
    return new.kind {
        id = look.id,
        pool = "effect",
        state = { ticks = "u8" },
        -- `sub_80E76F8`.
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                -- sub_80E7718: animation 0xB, stepped once, its palette (EXE4's: on its panel, at its height); on at
                -- once.
                local s = me.sprite
                s:load(look.sprite)
                s.shadow = "with_sprite"
                me.anim, me.anim_loaded = ANIM, ANIM
                s:set_animation(ANIM)
                s:step()
                me.visible = true
                s.palette = look.palette
                if look.height then
                    me:set_coordinates_from_panel()
                    local p = me.pos
                    me.pos = Vec3.new(p.x, p.y, look.height)
                end
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
