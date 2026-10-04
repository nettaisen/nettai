--!strict
-- Jealousy's dimming controller (BN5's effect object #0x36, 0x080E443C; BN6's code, `sub_80E440C`, which no BN6 chip
-- uses): the usual dimming phases (@exelib/dimming), with an effect (0x080E4480) that counts the chips the other
-- side holds (the most any of its actors holds: a player's hand from its cursor, another actor's own count, 0x080E4562)
-- and, once for each, every 10 ticks, hits every panel the other side stands on (0x080E45CC: region 1, the heavy
-- spark, hit modifier 3, the chip's damage); with none held, or once no panel had one of them, it waits 90 ticks. In
-- the battle flag 0x40 mode it warns the other side's player over the gauge for its first 70 of those (0x080E4642),
-- and then empties that side's custom gauge (0x080E4628).

local collision = require("../../rules/collision")
local dimming_chips = require("@exelib/dimming")
local regions = require("@exelib/regions")
local sparks = require("../../lib/sparks")

local controller = {}

export type State = {
    -- Hits left (+0x60: the chips counted).
    left: number,
    -- The chip's Atk+ bonus (+0x32), which adds to the hits' damage word.
    bonus: number,
}

-- The panel flag that says something of the other side's stands there (0x080E4618, by the controller's side).
local OCCUPIED = { [0] = 0x0020_0000, [1] = 0x0040_0000 }
local BETWEEN, WAIT, WARN_UNTIL = 0x0A, 0x5A, 0x14
local HIT_MOD = 3
local WARNING = asset.sound("bn5:err-select-91")
-- What the gauge loses in the battle flag 0x40 mode: all of it.
local FULL_GAUGE = 0x4000

-- 0x080E4596: the chips `o` holds: a player its side's hand from the cursor, another actor its own count (+0x1A).
local function held(o: Object): number
    if o.actor_type == "player" then
        return battle.hand_left(o.alliance)
    end
    return o.chips_held
end

-- 0x080E4562: the most chips any of the other side's actors holds.
local function most_held(me: Object): number
    local most = 0
    local other = bit32.bxor(me.alliance, 1)
    for i = 1, 4 do
        local o = battle.alive_actor_slot(other, i)
        if o then
            local n = held(o)
            if n >= most then
                most = n
            end
        end
    end
    return most
end

-- 0x080E45CC: a hit on each panel the other side stands on, columns 6 to 1, rows 3 to 1; how many.
local function hit_all(me: Object): number
    local flag = OCCUPIED[me.alliance]
    local word = dimming_chips.damage_plus(me, (me.state :: State).bonus)
    local n = 0
    for x = 6, 1, -1 do
        for y = 3, 1, -1 do
            if field.check(x, y, flag, 0) then
                -- 0x080C4DC6: a hit that runs while dimmed.
                local box = battle.hitbox(me, {
                    panel_x = x,
                    panel_y = y,
                    element = me.element,
                    z = 0,
                    region = regions.single,
                    hit_effect = sparks.heavy,
                    target = collision.hits_navis,
                    self_type = collision.collision_1a,
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

-- 0x080E4480, by its phase.
local function effect(me: Object)
    local s = me.state :: State
    local phase = me.phase
    if phase == 0 then
        -- 0x080E44A0: (its gauge pictures loaded) the count; with any, the hits from the next tick on.
        local n = most_held(me)
        s.left = n
        if n ~= 0 then
            me.timer = 1
            me.phase = 4
        else
            me.phase = 8
        end
        me.phase_init = 0
    elseif phase == 4 then
        -- 0x080E44EC: every 10 ticks the hits, a chip spent each; done when they run out or hit nowhere.
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
        -- 0x080E4510: 90 ticks, warning the other side for the first 70 (battle flag 0x40); then that side's gauge
        -- empties (battle flag 0x40) and the undim.
        if me.phase_init == 0 then
            me.timer = WAIT
            me.phase_init = 4
        end
        if me.timer >= WARN_UNTIL and battle.per_player_gauges() then
            -- 0x080E4642: on the consoles of the side this isn't (`battle_networkInvert`), over the gauge.
            battle.warn(WARNING, nil, bit32.bxor(me.alliance, 1))
        end
        local t = me.timer - 1
        me.timer = t
        if t >= 0 then
            return
        end
        if battle.per_player_gauges() then
            battle.drain_side_gauge(bit32.bxor(me.alliance, 1), FULL_GAUGE)
        end
        dimming_chips.done(me)
    else
        error(string.format("Jealousy's controller: phase %#x of its effect reads past its table (0x080E4494)", phase))
    end
end

controller.kind = define.kind {
    id = "bn5:jealousy/controller",
    pool = "effect",
    state = { left = "u32", bonus = "u16" },
    -- 0x080E443C.
    update = dimming_chips.phases { name = "Jealousy", effect = effect },
}

-- 0x080E4540: Jealousy's `dimming` hook (only the alliance byte of its user's).
function controller.hook(user: Object, spec: DimmingChipSpec): Object?
    local o = dimming_chips.spawn(controller.kind, user, spec, true)
    if o then
        (o.state :: State).bonus = spec.bonus
    end
    return o
end

return controller
