--!strict
-- Geddon's quake, as BN5 and BN6 share it (BN6's effect object #0x1E, `sub_80E25D0`; BN5's #0x1E, the same code;
-- BN6's docs/engine/dimming-chip-effects.md §12): it lists the field's solid panels, shuffles them, and changes one
-- every 8 ticks, the last listed first (Geddon breaks them, the capsules crack them, a third change no chip uses
-- poisons them), with a puff over each that changed; then it lets its holder go on. Geddon's and the capsules'
-- controller (@common/geddon/controller) spawns it.
--
-- Each game makes its own kind (`quake.make`) with its look: the puff, the bubble kind and the sounds.

local panels = require("@common/panels")
local rising_bubble = require("@common/rising-bubble/rising_bubble")

local quake = {}

-- What it does to a panel (`byte_80E2588`'s rows, by the chip's first
-- parameter): `object_crackPanelDup1` (crack a solid panel, or break a
-- cracked one nothing stands on), `object_breakPanel_dup3` (break a solid
-- panel, or crack it while something stands on it) or
-- `object_panel_setPoison` (a solid panel turns to poison).
export type Change = "crack" | "break" | "poison"

export type State = {
    change: Change,
    -- The panels listed (the object's +0x68, packed y << 4 | x), and how
    -- many are left (ExtraVars+4, a word that goes negative at the end).
    list: { number },
    count: number,
    -- The object whose `busy` it clears when it goes (the original's
    -- pointer to its controller's second parameter).
    holder: Object?,
}

export type Look = {
    -- The kind's id.
    id: string,
    -- Over a broken or cracked panel, a puff (effect 0x02); over a poisoned one a bubble rising (the game's rising
    -- bubble kind, palette 1).
    puff: Effect,
    bubble: Kind,
    sounds: { puff: Sound, bubble: Sound },
}

export type Quake = {
    kind: Kind,
    -- `sub_80E2712`: a quake of `change` on `spawner`'s side; `holder` (an object with a boolean `busy`) goes on
    -- once it is done.
    spawn: (spawner: Object, change: Change, holder: Object) -> Object?,
}

-- Every row's panels (`byte_80E2588` +4, +8): solid ones. (Every row
-- lists the whole field, `sub_80E269A`; the row builder `sub_80E26D8`,
-- its own row's, is in no row.)
local SOLID = 0x10
-- Ticks between the changes, the first on the quake's eighth update.
local PERIOD = 8
local BUBBLE_PALETTE = 1

-- The panel routine (`byte_80E2588` +0x14): whether the panel changed.
-- Each plays its own sound when it does.
local function change_panel(change: Change, x: number, y: number): boolean
    if change == "crack" then
        return field.crack(x, y)
    elseif change == "break" then
        return field.break_panel(x, y)
    end
    return field.poison(x, y)
end

function quake.make(look: Look): Quake
    -- `sub_80E2628`: every 8 ticks the next panel, from the list's end; once
    -- the list is spent, destroy (a byte store: the destroy routine runs at
    -- the next update).
    local function tick(me: Object)
        local t = me.timer - 1
        me.timer = t
        if t ~= 0 then
            return
        end
        me.timer = PERIOD
        local s = me.state :: State
        s.count -= 1
        if s.count < 0 then
            me.lifecycle = "destroy"
            return
        end
        local p = s.list[s.count + 1]
        local x, y = panels.x(p), panels.y(p)
        if not change_panel(s.change, x, y) then
            return
        end
        local cx, cy = field.center(x, y)
        local at = Vec3.new(cx, cy, 0)
        if s.change == "poison" then
            rising_bubble.spawn(look.bubble, at, BUBBLE_PALETTE)
            battle.play_sound(look.sounds.bubble)
        else
            battle.effect(at, look.puff)
            battle.play_sound(look.sounds.puff)
        end
    end

    -- `sub_80E25F0`: list the field's solid panels (`sub_80E269A`: rows 3 to
    -- 1, columns 6 to 1) and shuffle them (`sub_8000C72`, as many swaps as
    -- panels: two draws each); the timer; the first update at once.
    local function init(me: Object)
        local s = me.state :: State
        local list = panels.filtered(SOLID, 0)
        if #list > 0 then
            panels.shuffle(list, #list, #list)
        end
        for i, p in list do
            s.list[i] = p
        end
        s.count = #list
        me.timer = PERIOD
        me.lifecycle = "update"
        tick(me)
    end

    local kind = define.kind {
        id = look.id,
        pool = "effect",
        state = { change = { "crack", "break", "poison" }, list = "u8[18]", count = "i32", holder = "object" },
        -- `sub_80E25D0`.
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                init(me)
            elseif l == "update" then
                tick(me)
            else
                -- sub_80E268C: the holder goes on (the original clears the
                -- byte wherever it points); freed.
                local holder = (me.state :: State).holder
                if holder and holder.active then
                    (holder.state :: { busy: boolean }).busy = false
                end
                me:free()
            end
        end,
    }

    -- `sub_80E2712`: (Spawned at the spawner's registers, which nothing reads; its panel is none.)
    local function spawn(spawner: Object, change: Change, holder: Object): Object?
        local o = battle.spawn(kind, Vec3.zero)
        if o then
            local s = o.state :: State
            s.change, s.holder = change, holder
            o.alliance = spawner.alliance
        end
        return o
    end

    return { kind = kind, spawn = spawn }
end

return quake
