--!strict
-- PileDrvr's dimming controller (BN5's effect object #0x6F, 0x080E8800; BN6's code, `sub_80E83B8`, which no BN6 chip
-- uses): the usual dimming phases (@exelib/dimming), with an effect (0x080E8844) that sets down a pile (./pile) on
-- the panel ahead of the user and another, turned around, at the far end of the user's row, if those are solid with
-- nothing on them (0x080E8918), each knowing how far past it the first of the other side's bodies stands in the row
-- (0x080E88CE). Someone four or more panels past the first pile means no piles at all, as does a first panel that
-- isn't free; a second pile that sees no one, or only someone that far, beams at no one. The effect waits until
-- its piles are done; then the undim.

local dimming_chips = require("@exelib/dimming")
local pile = require("./pile")

export type State = {
    -- Its piles at work (+0x06 and +0x07, the bytes each pile clears when done; the hook's r4, the chip record's
    -- parameters, starts them: PileDrvr's are 0).
    first_busy: boolean,
    second_busy: boolean,
    -- The chip's Atk+ bonus (+0x32), which adds to the piles' damage word.
    bonus: number,
}

local controller = {}

-- A pile's panel: solid (0x10), with nothing on it (0x0F880080: 0x080E89C0).
local SOLID, TAKEN = 0x10, 0x0F88_0080
-- The other side's bodies, by the side looking (0x080E8910).
local BODIES = { [0] = 0x0580_0000, [1] = 0x0A80_0000 }
-- No one in the row (0xFF), and the nearest the first pile may see someone from.
local NONE, REACH = 0xFF, 4

-- `object_getFlipDirection`: +1 for a side and flip that face right, else -1.
local function direction(alliance: number, flip: number): number
    return if bit32.bxor(alliance, flip) == 0 then 1 else -1
end

-- 0x080E88CE: how many panels past (x, y), looking `alliance` and `flip`'s way, the first panel with one of that
-- side's opponents' bodies on it is (0 for the next one); NONE if the field ends first.
local function distance(x: number, y: number, alliance: number, flip: number): number
    local bodies = BODIES[alliance]
    local step = direction(alliance, flip)
    local n = 0
    x += step
    while field.valid(x, y) do
        if bit32.btest(field.flags(x, y), bodies) then
            return n
        end
        n += 1
        x += step
    end
    return NONE
end

-- What 0x080E8918 finds: the piles' panels (a second X of 0: none) and how far each sees.
type Piles = { x: number, y: number, distance: number, x2: number, y2: number, distance2: number }

-- 0x080E8918: the panel ahead of the controller, and the far end of its row (column 6 for a side facing right, 1
-- for one facing left) if that is another panel; each one where a pile can stand. Nil for no piles.
local function find(me: Object): Piles?
    local x, y = me.panel_x + me.facing, me.panel_y
    if not field.check(x, y, SOLID, TAKEN) then
        return nil
    end
    local found: Piles = { x = x, y = y, distance = 0, x2 = 0, y2 = 0, distance2 = 0 }
    local far = 6 - 5 * bit32.bxor(me.alliance, me.flip)
    -- (In the first's row, so another panel if another column.)
    if field.check(far, y, SOLID, TAKEN) and far ~= x then
        found.x2, found.y2 = far, y
    end
    found.distance = distance(x, y, me.alliance, me.flip)
    if found.distance ~= NONE and found.distance >= REACH then
        return nil
    end
    if found.x2 == 0 then
        return found
    end
    -- The second looks back along the row.
    local d = distance(found.x2, found.y2, me.alliance, bit32.bxor(me.flip, 1))
    if d ~= NONE and (found.distance == NONE or d >= REACH) then
        d = NONE
    end
    found.distance2 = d
    return found
end

-- 0x080E8844: on its first tick the piles, the second turned around (the chip's damage word plus its bonus); then
-- the undim once neither is at work.
local function effect(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        local found = find(me)
        if found then
            local word = dimming_chips.damage_plus(me, s.bonus)
            if pile.spawn(me, 0, found.x, found.y, me.element, s.bonus, word, found.distance) then
                s.first_busy = true
            end
            if found.x2 ~= 0 then
                local second = pile.spawn(me, 1, found.x2, found.y2, me.element, s.bonus, word, found.distance2)
                if second then
                    s.second_busy = true
                    second.flip = bit32.bxor(second.flip, 1)
                end
            end
        end
        me.phase_init = 4
    end
    if not s.first_busy and not s.second_busy then
        dimming_chips.done(me)
    end
end

controller.kind = define.kind {
    id = "piledrvr/controller",
    pool = "effect",
    state = { first_busy = "bool", second_busy = "bool", bonus = "u16" },
    -- 0x080E8800.
    update = dimming_chips.phases { name = "PileDrvr", effect = effect },
}

-- 0x080E88AC: PileDrvr's `dimming` hook (its user's side and facing).
function controller.hook(user: Object, spec: DimmingChipSpec): Object?
    local o = dimming_chips.spawn(controller.kind, user, spec)
    if o then
        (o.state :: State).bonus = spec.bonus
    end
    return o
end

return controller
