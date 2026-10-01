--!strict
-- Where an owner keeps an attached object (the gun, the sun beam). The
-- object lives while the slot holds something, not necessarily the object
-- itself; the owner ends it by clearing the slot.

-- (The `Slot` type is declared for the pack in types.d.luau.)

local slot = {}

-- A content owner's state with a `held` slot (an ExtraVars word the game
-- points its attachment at), and `held_2` for an owner that keeps a second
-- (DblBeast's Gregar, DblHero's MegaMan).
export type Holder = { held: Object?, held_2: Object? }

-- The owner's actor overlay slot, its first related object, or its
-- state's `held` (or `held_2`) field.
function slot.occupied(owner: Object, which: Slot): boolean
    if which == "overlay" then
        return owner.overlay ~= nil
    elseif which == "held" then
        return (owner.state :: Holder).held ~= nil
    elseif which == "held_2" then
        return (owner.state :: Holder).held_2 ~= nil
    end
    return owner.related1 ~= nil
end

function slot.fill(owner: Object, which: Slot, o: Object?)
    if which == "overlay" then
        owner.overlay = o
    elseif which == "held" then
        (owner.state :: Holder).held = o
    elseif which == "held_2" then
        (owner.state :: Holder).held_2 = o
    else
        owner.related1 = o
    end
end

-- The declared type of a slot field in a kind's state.
slot.TYPE = { "overlay", "related", "held", "held_2" }

return slot
