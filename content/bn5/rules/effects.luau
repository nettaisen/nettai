--!strict
-- BN5's shared battle effects where they touch the simulation otherwise than BN6's (docs/design/bn5-map.md §15.3
-- items 15 and 16), the arena's: its camera shake (0x08030D78) has one channel and draws its jitter from the
-- battle's RNG2, twice a shaking tick, holding while the battle is paused without dimming (BN6's draws from each
-- console's RNG1); its hit spark (0x080E0870) doesn't step its sprite as it starts, so it lasts a tick longer than
-- BN6's (`sub_80E0864`).

return define.rules("effects", {
    shake = "battle",
    spark_steps_at_start = false,
})
