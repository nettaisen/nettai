--!strict
-- Jealousy's dimming controller (EXE6's effect object #0x36, `sub_80E440C`, which no EXE6 chip uses; EXE5's #0x36,
-- 0x080E443C; EXE4's 0x080E5EF4), made for a game's look (`controller.make`): the usual dimming phases (@exelib/dimming),
-- with an effect that counts the chips the other side holds (the look's `count`) and, once for each, every 10 ticks,
-- hits every panel the other side stands on (`sub_80E459C`: region 1, the look's spark, hit modifier 3, the chip's
-- damage and the telop's bonus, running while dimmed); with none held, or once no panel had one of them, it waits 90
-- ticks. EXE5's adds the operation battle's warning and emptied gauge (the look's `operation_battle`).
--
-- Its spawn (`sub_80E4510`) takes the user's side alone.

local dimming_chips = require("../dimming")
local regions = require("../regions")

local controller = {}

export type State = {
    -- Hits left (+0x60: the chips counted).
    left: number,
    -- The chip's Atk+ bonus (+0x32), which adds to the hits' damage word.
    bonus: number,
}

-- The panel flag that says something of the other side's stands there (`byte_80E4618`, by the controller's side).
local OCCUPIED = { [0] = 0x0020_0000, [1] = 0x0040_0000 }
local BETWEEN, WAIT, WARN_UNTIL = 0x0A, 0x5A, 0x14
local HIT_MOD = 3
-- What the gauge loses in the operation battle: all of it.
local FULL_GAUGE = 0x4000

-- A game's Jealousy: the chip's `dimming` hook.
function controller.make(look: JealousyLook): (user: Object, spec: DimmingChipSpec) -> Object?
    -- `sub_80E459C`: a hit on each panel the other side stands on, columns 6 to 1, rows 3 to 1; how many.
    local function hit_all(me: Object): number
        local flag = OCCUPIED[me.alliance]
        local word = dimming_chips.damage_plus(me, (me.state :: State).bonus)
        local n = 0
        for x = 6, 1, -1 do
            for y = 3, 1, -1 do
                if field.check(x, y, flag, 0) then
                    -- `sub_80C53A6`: a hit that runs while dimmed.
                    local box = battle.hitbox(me, {
                        panel_x = x,
                        panel_y = y,
                        element = me.element,
                        z = 0,
                        region = regions.single,
                        hit_effect = look.spark,
                        target = look.hits,
                        self_type = look.is,
                        damage = bit32.band(word, 0xFFFF),
                        stamina = bit32.rshift(word, 16),
                        hit_mod = HIT_MOD,
                    })
                    if box then
                        box.run_while_dimmed = true
                    end
                    n += 1
                end
            end
        end
        return n
    end

    -- `sub_80E4450`, by its phase.
    local function effect(me: Object)
        local s = me.state :: State
        local phase = me.phase
        if phase == 0 then
            -- `sub_80E4470`: the count; with any, the hits from the next tick on.
            local n = look.count(me)
            s.left = n
            if n ~= 0 then
                me.timer = 1
                me.phase = 4
            else
                me.phase = 8
            end
            me.phase_init = 0
        elseif phase == 4 then
            -- `sub_80E44BC`: every 10 ticks the hits, a chip spent each; done when they run out or hit nowhere.
            local t = me.timer - 1
            me.timer = t
            if t ~= 0 then
                return
            end
            me.timer = BETWEEN
            if hit_all(me) == 0 then
                me.phase, me.phase_init = 8, 0
                return
            end
            s.left -= 1
            if s.left == 0 then
                me.phase, me.phase_init = 8, 0
            end
        elseif phase == 8 then
            -- `sub_80E44E0`: 90 ticks; in EXE5's operation battle warning the other side for the first 70, then
            -- emptying that side's gauge; then the undim.
            if me.phase_init == 0 then
                me.timer = WAIT
                me.phase_init = 4
            end
            local operation = look.operation_battle
            if operation and me.timer >= WARN_UNTIL and operation() then
                -- (EXE5's 0x080E4642: on the consoles of the side this isn't, over the gauge.)
                battle.warn(look.warning :: Sound, nil, bit32.bxor(me.alliance, 1))
            end
            local t = me.timer - 1
            me.timer = t
            if t >= 0 then
                return
            end
            if operation and operation() then
                battle.drain_side_gauge(bit32.bxor(me.alliance, 1), FULL_GAUGE)
            end
            dimming_chips.done(me)
        else
            error(string.format("Jealousy's controller: phase %#x of its effect reads past its table", phase))
        end
    end

    local kind = new.kind {
        id = look.id,
        pool = "effect",
        state = { left = "u32", bonus = "u16" },
        -- `sub_80E440C`.
        update = dimming_chips.phases { name = "Jealousy", effect = effect },
    }

    -- `sub_80E4510`: the chip's `dimming` hook (only the alliance byte of its user's).
    return function(user: Object, spec: DimmingChipSpec): Object?
        local o = dimming_chips.spawn(kind, user, spec, true)
        if o then
            (o.state :: State).bonus = spec.bonus
        end
        return o
    end
end

return controller
