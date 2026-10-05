--!strict
-- EXE6's chip gate battle (battle flag 0x40, the engine's `OWN_GAUGES`; set once a battle by `sub_802E112` when a chip
-- gate is on the link port, 0x0200AD04, or in a link battle whose consoles both have one, EVENT_1722): each side its
-- own slot-in gauge, which pays for the chips slotted into the gate; SELECT's Program Advance window; MegaMan in Beast
-- Out, with no Cross or Beast Out buttons. Never in a netbattle without gates.
return function(): boolean
    return battle.own_gauges()
end
