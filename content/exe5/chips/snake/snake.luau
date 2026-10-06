--!strict
-- EXE5's Snake (the dimming handler's subtype 16): EXE6's controller (effect object #0x32, EXE6's #0x45, the same code:
-- the dimming's phases, a nest on the user's panel, 41 ticks once it is done) with EXE5's nest and snakes.
--
-- - The nest (attack object #0x53, 0x080D03E4, EXE6's #0x72 reworked) scans the user's side's holes from the far
--   column back (the shared scan, @exelib/snake/snake) and sends snakes out of up to three at once, each with its
--   flag (ExtraVars+4: a byte each); when every flag is clear again it sends the next three; once the holes run out
--   and the flags are clear, it goes.
-- - The snake (attack object #0x54, EXE6's #0x73: the shared snake) waits 48 ticks (EXE6 20), strikes as wood (EXE6
--   null) with the wood spark (spark 4, EXE6's 5) and clears its flag when it goes.

local collision = require("../../rules/collision")
local common = require("@exelib/snake")
local trajectory = require("../../lib/trajectory")
local dimming_chips = require("@exelib/dimming")
local sparks = require("../../lib/sparks")

local snake = {}

-- ---- The snake ---------------------------------------------------------------------------------------------------

export type NestState = {
    scan_x: number,
    scan_y: number,
    -- The snakes' flags (ExtraVars+4..+7): set while each is out.
    flags: number,
    -- The holder whose `busy` it clears when it goes.
    holder: Object?,
}

snake.kind = common.make_kind("snake/snake", {
    trajectory = trajectory,
    sprite = asset.sprite("snake"),
    rise_sound = asset.sound("appear"),
    leap_sound = asset.sound("aqua-needle-2"),
    wait = 0x30,
    strike_element = 4,
    strike_spark = sparks.wood,
    strike_hit_mod = 3,
    is = collision.attack,
    hits = collision.hits_navis,
    joined = function(_me: Object) end,
    -- 0x080D07D8: its flag cleared.
    left = function(me: Object)
        local s = me.state :: SnakeState
        local nest = s.nest
        if nest and nest.active then
            local n = nest.state :: NestState
            n.flags = bit32.band(n.flags, bit32.bnot(bit32.lshift(0xFF, 8 * s.slot)))
        end
    end,
})

-- ---- The nest ----------------------------------------------------------------------------------------------------

-- Snakes a batch.
local BATCH = 3

-- 0x080D03E4: the scan's start; while no snake is out, up to three snakes; once the holes run out and none is out,
-- go.
local function nest_tick(me: Object)
    local s = me.state :: NestState
    if me.phase_init == 0 then
        me.timer2 = 0
        s.scan_x = 6 - 5 * bit32.bxor(me.alliance, me.flip)
        s.scan_y = 1
        me.phase_init = 4
    end
    if s.flags ~= 0 then
        return
    end
    while true do
        local hx: number?, hy: number, tx: number, ty: number
        if field.valid(s.scan_x, s.scan_y) then
            hx, hy, tx, ty = common.next_hole(me, s)
        end
        if not hx then
            break
        end
        local slot = me.timer2
        s.flags = bit32.bor(s.flags, bit32.lshift(1, 8 * slot))
        local o = common.spawn(snake.kind, me, hx, hy, tx, ty, slot)
        if o then
            o.run_while_dimmed = true
        end
        local next_slot = slot + 1
        me.timer2 = next_slot
        if next_slot >= BATCH then
            me.timer2 = 0
            return
        end
    end
    if s.flags == 0 then
        me:set_lifecycle("destroy")
    end
end

snake.nest = new.kind {
    id = "snake/nest",
    pool = "attack",
    state = { scan_x = "u8", scan_y = "u8", flags = "u32", holder = "object" },
    -- 0x080D0380 (EXE6's `sub_80D2BDC`).
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            me:set_coordinates_from_panel()
            me:set_lifecycle("update")
            nest_tick(me)
        elseif l == "update" then
            nest_tick(me)
        else
            -- The holder goes on; freed.
            local holder = (me.state :: NestState).holder
            if holder and holder.active then
                (holder.state :: { busy: boolean }).busy = false
            end
            me:free()
            return
        end
        me:update_sprite()
    end,
}

-- A nest on (`x`, `y`) of `controller`'s side and facing, with the damage word `damage`, running while dimmed;
-- `holder` goes on once it is done. (Its Z stays `z`, the controller's bonus as a raw 16.16 value.)
local function spawn_nest(controller: Object, x: number, y: number, z: number, damage: number, holder: Object): Object?
    local o = battle.spawn(snake.nest, Vec3.new(0, 0, z))
    if o then
        o.panel_x, o.panel_y = x, y
        o:set_damage_word(damage)
        o.related1 = controller
        local s = o.state :: NestState
        s.holder = holder
        o.alliance, o.flip = controller.alliance, controller.flip
        o.run_while_dimmed = true
    end
    return o
end

-- ---- The controller ----------------------------------------------------------------------------------------------

export type ControllerState = {
    -- The Atk+ / cross bonus (object +0x32): it adds to the snakes' damage word.
    bonus: number,
    -- The nest runs.
    busy: boolean,
}

local AFTER = 0x28

-- 0x080E40BC (EXE6's `sub_80E5988`): the nest; then, once it is done, 41 ticks.
local function effect(me: Object)
    local s = me.state :: ControllerState
    if me.phase_init == 0 then
        spawn_nest(me, me.panel_x, me.panel_y, s.bonus, dimming_chips.damage_plus(me, s.bonus), me)
        s.busy = true
        me.timer = AFTER
        me.phase_init = 4
    end
    if s.busy then
        return
    end
    local t = me.timer - 1
    me.timer = t
    if t < 0 then
        dimming_chips.done(me)
    end
end

snake.controller = new.kind {
    id = "snake/controller",
    pool = "effect",
    state = { bonus = "u16", busy = "bool" },
    update = dimming_chips.phases { name = "Snake", effect = effect },
}

-- 0x080E40FA (EXE6's `sub_80E59C6`): Snake's `dimming` hook.
function snake.hook(user: Object, spec: DimmingChipSpec): Object?
    local o = dimming_chips.spawn(snake.controller, user, spec)
    if o then
        (o.state :: ControllerState).bonus = spec.bonus
    end
    return o
end

return snake
