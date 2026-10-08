--!strict
-- Geddon's quake, as EXE5 and EXE6 share it (EXE6's effect object #0x1E, `sub_80E25D0`; EXE5's #0x1E, the same code;
-- EXE6's docs/engine/dimming-chip-effects.md §12): it lists the field's solid panels, shuffles them, and changes one
-- every 8 ticks, the last listed first, as its chip's change says (`quake.change`: what it does to a panel, and what
-- shows over each that changed; the original's rows of `byte_80E2588` by the chip's first parameter): Geddon breaks
-- them, the capsules crack them (a puff over each), EXE5's Geddon3 poisons them (bubbles). Then it lets its holder go
-- on. Geddon's and the capsules' controller (@exelib/geddon/controller) spawns it.
--
-- Each game makes its own kind (`quake.make`) with its look: the puff, the bubble kind and the sounds; the made quake
-- gives the shows and the poisoning with that look (`puff`, `bubbles`, `poison`).

local panels = require("@exelib/panels")
local rising_bubble = require("@exelib/rising_bubble")

local quake = {}

-- A chip's change (`GeddonChange`): what it does to a panel (the
-- original's row's routine, `byte_80E2588` +0x14: `object_crackPanelDup1`,
-- crack a solid panel or break a cracked one nothing stands on,
-- `field.crack`; `object_breakPanel_dup3`, break a solid panel or crack it
-- while something stands on it, `field.break_panel`;
-- `object_panel_setPoison`, a solid panel turns to poison, the made
-- quake's `poison`), whether it changed; and what shows over a panel that
-- did (the made quake's `puff` or `bubbles`).
export type Change = GeddonChange

function quake.change(c: Change): Change
    return new.record("geddon-change", c)
end

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
    -- The sounds: the puff's, the bubble's, and a panel turning to poison's (`object_panel_setPoison`'s).
    sounds: { puff: Sound, bubble: Sound, poison: Sound },
}

export type Quake = {
    kind: Kind,
    -- `sub_80E2712`: a quake of `change` on `spawner`'s side; `holder` (an object with a boolean `busy`) goes on
    -- once it is done.
    spawn: (spawner: Object, change: Change, holder: Object) -> Object?,
    -- A chip's change (`quake.change`), and the shows and the poisoning with the look (see `Change`).
    change: (c: Change) -> Change,
    puff: (at: Vec3) -> (),
    bubbles: (at: Vec3) -> (),
    poison: (x: number, y: number) -> boolean,
}

-- Every row's panels (`byte_80E2588` +4, +8): solid ones. (Every row
-- lists the whole field, `sub_80E269A`; the row builder `sub_80E26D8`,
-- its own row's, is in no row.)
local SOLID = 0x10
-- Ticks between the changes, the first on the quake's eighth update.
local PERIOD = 8
local BUBBLE_PALETTE = 1

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
        if not s.change.apply(x, y) then
            return
        end
        local cx, cy = field.center(x, y)
        s.change.show(Vec3.new(cx, cy, 0))
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

    local kind = new.kind {
        id = look.id,
        pool = "effect",
        state = { change = "record:geddon-change", list = "u8[18]", count = "i32", holder = "object" },
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

    -- A puff over a changed panel, with its sound (a crack's, a break's).
    local function puff(at: Vec3)
        battle.effect(at, look.puff)
        battle.play_sound(look.sounds.puff)
    end

    -- Bubbles rising over a changed panel, with their sound (a poisoning's).
    local function bubbles(at: Vec3)
        rising_bubble.spawn(look.bubble, at, BUBBLE_PALETTE)
        battle.play_sound(look.sounds.bubble)
    end

    -- `object_panel_setPoison`: a solid panel turns to poison, with its sound.
    local function poison(x: number, y: number): boolean
        return field.overwrite(x, y, "poison", look.sounds.poison)
    end

    return { kind = kind, spawn = spawn, change = quake.change, puff = puff, bubbles = bubbles, poison = poison }
end

return quake
