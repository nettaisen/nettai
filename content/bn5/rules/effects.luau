--!strict
-- BN5's shared battle effects where they touch the simulation otherwise than BN6's (docs/design/bn5-map.md §15.3
-- items 15 and 16), the arena's: its camera shake (0x08030D78) has one channel and draws its jitter from the
-- battle's RNG2, twice a shaking tick, holding while the battle is paused without dimming (BN6's draws from each
-- console's RNG1); its hit spark (0x080E0870) doesn't step its sprite as it starts, so it lasts a tick longer than
-- BN6's (`sub_80E0864`); setting an object's collision types again (0x08016B9E, BN6's `sub_801A082`) sets what it
-- is alone, never marked as made while dimmed (its store of what it hits goes to a BIOS address). Its obstacles'
-- reactions have the step for ColonelSoul's army (0x080CAB02 from 0x08018000, 0x08018168, 0x080182D4 and 0x08018404;
-- bn5-map.md §15.11): an obstacle where an armed side can use it turns into its soldier (objects/soldier, the role
-- `kinds.obstacle_soldier`).

return define.rules("bn5:effects", {
    shake = "battle",
    spark_steps_at_start = false,
    retype = "bn5",
    obstacle_soldiers = true,
})
