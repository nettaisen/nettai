--!strict
-- BN5's operation battle (battle flag 0x40, the engine's `OWN_GAUGES`; BN5 sets it at 0x0802D590 when the battle
-- mode isn't 1 and the navi's stats' +0x2A is set): both navis computer-driven, the Tactics screen, each side its own
-- custom gauge. Never in a netbattle.
return function(): boolean
    return battle.own_gauges()
end
