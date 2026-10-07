--!strict
-- What the panel types the games share do, as their hooks (a panel type's `stand`, `burn`, `rest`, `move_end`,
-- `slide`: docs/design/rules-in-luau.md, "Panels into Luau"): poison's drain, grass's heal, a drain of a body of one
-- element (EXE5's sea), a burn (EXE5's and EXE4's lava), a road's carry, EXE5's magnet's slide and sea's hold, ice's
-- slide or push, EXE4's pitfall's stop. Each game's rules/panels.luau gives its types these, with its own numbers.

local element = require("./element")

local panel_types = {}

local FIRE = element.byte("fire")
local AQUA = element.byte("aqua")
local WOOD = element.byte("wood")
-- The damage slot a drain counts in (the sixth: `sub_801A186`'s +0x8C).
local DRAIN = 5
-- The ticks between a drain's points: the count starts again at 6.
local DRAIN_EVERY = 6

-- `sub_801A186`'s drain (EXE5's 0x08016C7E, EXE4's 0x08012FF2): a point every seventh tick a body stands on the panel,
-- through the drain's slot, unless it is untouchable, floats or is invulnerable. (The game's grass test then reads
-- the body's status word where the panel's type was, against grass's number, 6: a word with one of those flags is
-- never 6, so an immune body never heals there; its count goes to 0.)
local function drain(me: Object)
    local c = me.collision
    if me:status("untouchable") or me:status("float_shoe") or me:status("invulnerable") then
        c.standing_count = 0
        return
    end
    local left = c.standing_count - 1
    if left < 0 then
        c.standing_count = DRAIN_EVERY
        c:add_damage(DRAIN, 1, false)
    else
        c.standing_count = left
    end
end

-- Poison's `stand`: the drain.
function panel_types.poison(me: Object)
    drain(me)
end

-- A `stand` that drains a body of `of` as poison drains any, and leaves another be (EXE5's sea, fire bodies:
-- 0x08016C7E).
function panel_types.drains(of: string): (Object) -> ()
    local e = element.byte(of)
    return function(me: Object)
        if me.collision.element == e then
            drain(me)
        else
            me.collision.standing_count = 0
        end
    end
end

-- Grass's `stand` (`sub_801A186`'s end): a wood body heals a point when the battle's 20-tick count comes round; at
-- `slows_at` HP or less (EXE6's and EXE5's 9; EXE4's none, 0x08012FF2), its 180-tick count.
function panel_types.grass(slows_at: number?): (Object) -> ()
    return function(me: Object)
        me.collision.standing_count = 0
        if bit32.band(me.element, 0xF) ~= WOOD then
            return
        end
        local slow = slows_at ~= nil and me.hp <= slows_at
        if battle.cycle(if slow then 180 else 20) == 0 then
            me.hp = math.min(me.hp + 1, me.max_hp)
        end
    end
end

export type BurnSpec = {
    -- The damage in fire, shifted by the body's weakness to fire.
    damage: number,
    -- The statuses that spare a body (EXE5's 0x88000206, EXE4's 0x206).
    spared_by: { StatusFlag },
    -- What it wears off the mood besides (EXE4's 20, its +0x36).
    mood: number?,
    -- A player burns while the battle is dimmed too (EXE4's 0x08013128, which has no dimming test; every other
    -- body's waits, as EXE5's).
    players_while_dimmed: boolean?,
    -- What the burn shows (jittered by 15: one simulation RNG draw).
    spark: Spark,
}

-- A `burn` (EXE5's lava, 0x08016E18 for navis and 0x08016D80 for other bodies, the same tests in another order;
-- EXE4's, 0x08013128 and 0x0801309E): a grounded body on it that isn't of fire nor spared takes its damage in fire as
-- a hit, and its mood, unless guarded or invulnerable; either way the panel turns normal and the burn shows. Not
-- while the battle is dimmed, unless the body is a player's and the spec says so.
function panel_types.burn(spec: BurnSpec): (Object, boolean) -> ()
    return function(me: Object, player: boolean)
        if battle.dimmed() and not (player and spec.players_while_dimmed) then
            return
        end
        local c = me.collision
        if c.element == FIRE or c.region == nil or me:status("float_shoe") then
            return
        end
        for _, s in spec.spared_by do
            if me:status(s) then
                return
            end
        end
        if not me:status("guard") and not me:status("invulnerable") then
            local damage = bit32.band(bit32.lshift(spec.damage, battle.weakness(c.element, FIRE)), 0xFFFF)
            c:add_damage(FIRE, damage, true)
            c:add_mood_damage(spec.mood or 0)
            -- (A hit: the hit modifiers 1 and 2, on both sides'.)
            c:add_hit_mod(3)
        end
        local x, y = c.panel_x, c.panel_y
        field.set_type(x, y, "normal")
        local cx, cy = field.center(x, y)
        battle.spark(me, battle.jitter(0xF, Vec3.new(cx, cy, 0)), spec.spark)
    end
end

-- A slide a panel starts: the body's slide request, of type `kind` ("rows": by the arena's rows and the move's
-- direction; "panel": the panel's own carry).
local function request_slide(me: Object, kind: SlideType)
    me:set_request("slide", true)
    me.slide_type = kind
end

-- Floating or submerged (the status word's 0x24, which the slides' tests share).
local function off_the_floor(me: Object): boolean
    return me:status("float_shoe") or me:status("submerged")
end

-- A road's `rest` (`sub_801A36A`'s first test, `sub_801A400`): a body at rest on it is carried the road's way (slide
-- type 3, "panel") once its slide cooldown is out, unless floating or submerged; either way the rest of the tests
-- wait (the move's end).
function panel_types.road(me: Object): boolean
    if me.slide_cooldown == 0 and not off_the_floor(me) then
        request_slide(me, "panel")
    end
    return true
end

-- The `slide` of a type that carries a body (EXE6's roads, EXE5's magnet: 0x08013564's type 5 where EXE6 tests 9
-- to 12): a slide that reaches it, not floating or submerged, is carried on its way.
function panel_types.carries(me: Object, how: SlideHow): SlideAnswer?
    if how == "slide" and not off_the_floor(me) then
        return "carry"
    end
    return nil
end

-- EXE5's magnet's `move_end` (0x08017216): a move's end on it slides the body (slide type 3, "panel": its tries by the
-- move's direction), unless the body is dragged or moving, slid within its cooldown, floats or is submerged, or is a
-- navi whose form stands on magnet panels (EXE5's MagnetSoul).
function panel_types.magnet(me: Object)
    if me:status("drag") or me:status("moving") then
        return
    end
    if me.slide_cooldown ~= 0 or off_the_floor(me) or me:form_trait("stands_on_magnet") then
        return
    end
    request_slide(me, "panel")
end

-- EXE5's sea's `move_end` (0x080171C2): a move's end on it holds the body there for `ticks` (immobilized: EXE6's
-- `sub_800EB18`) with `splash`, unless the body is dragged or moving, floats, dives or is of aqua.
function panel_types.holds(ticks: number, splash: Effect): (Object) -> ()
    return function(me: Object)
        if me:status("drag") or me:status("moving") then
            return
        end
        if me:status("float_shoe") or me:navi_state("dives") or me.element == AQUA then
            return
        end
        me:set_status_timer("immobilize", ticks)
        me:set_status("immobilized", true)
        battle.effect(me.pos, splash)
    end
end

-- EXE5's sea's `slide`: a slide stops on it.
function panel_types.stops_a_slide(_me: Object, how: SlideHow): SlideAnswer?
    return if how == "slide" then "stop" else nil
end

-- Ice's `move_end` (EXE6's `sub_801A3DA`, EXE5's 0x080171F0): a body not of aqua, not floating or submerged, that ice
-- affects, slides (slide type 2, "rows": the arena's rows by the move's direction).
function panel_types.ice_slides(me: Object)
    if me.collision.element ~= AQUA and not off_the_floor(me) and me:status("affected_by_ice") then
        request_slide(me, "rows")
    end
end

-- EXE4's ice's `move_end` (0x0801335A): the same body is pushed, a bit by its side and the move's direction ORed into
-- its final hit modifier (the table at 0x080133B4, five directions a side; a direction past a side's row pushes
-- nothing), which the intake reads after: a slide by the push's row, or a drag with a hit's drag bit.
function panel_types.ice_pushes(bits: { { number } }): (Object) -> ()
    return function(me: Object)
        if me.collision.element ~= AQUA and not off_the_floor(me) and me:status("affected_by_ice") then
            local row = bits[bit32.band(me.alliance, 1) + 1]
            me.collision:add_final_hit_mod(row[me.collision.direction + 1] or 0)
        end
    end
end

-- Ice's `slide`: a slide, a drag or an obstacle's slide that reaches it goes a panel further, unless of aqua.
function panel_types.ice(me: Object, _how: SlideHow): SlideAnswer?
    return if me.collision.element ~= AQUA then "on" else nil
end

-- EXE4's pitfall's `slide` (0x080102FC, 0x08010B54; an obstacle's, 0x080106B8 and its kin): a slide, a drag or an
-- obstacle's slide stops on it, unless the body floats.
function panel_types.pitfall(me: Object, _how: SlideHow): SlideAnswer?
    return if me:status("float_shoe") then nil else "stop"
end

return panel_types
