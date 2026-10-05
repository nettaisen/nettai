--!strict
-- The test content's SP navi chips: two made-up deletion times (BCD
-- hours:minutes:seconds.hundredths) at which an SP navi chip's damage
-- steps down, and the one slot a setup carries: Count[SP], which EXE6's
-- modules bring.

return {
    deletion_times = { 0x2000, 0x4000 },
    slots = { "sp/count" },
}
