--!strict
-- Where an owner keeps an attached object (the gun, the sun beam). The
-- object lives while the slot holds something, not necessarily the object
-- itself; the owner ends it by clearing the slot.

-- (The `Slot` type is declared for the pack in types.d.luau.)

local slot = {}

-- The owner's actor overlay slot, its first related object, or (for an
-- owner a script implements) its state's `attachment` field, which the
-- owner's kind declares as an object.
function slot.occupied(owner: Object, which: Slot): boolean
    if which == "overlay" then
        return owner.overlay ~= nil
    elseif which == "state" then
        return (owner.state :: any).attachment ~= nil
    end
    return owner.related1 ~= nil
end

function slot.fill(owner: Object, which: Slot, o: Object?)
    if which == "overlay" then
        owner.overlay = o
    elseif which == "state" then
        (owner.state :: any).attachment = o
    else
        owner.related1 = o
    end
end

-- The declared type of a slot field in a kind's state.
slot.TYPE = { "overlay", "related", "state" }

return slot
