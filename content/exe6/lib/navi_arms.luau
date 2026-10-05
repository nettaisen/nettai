--!strict
-- What a navi holds in its own look: the animation of a chip's cannon, air
-- shooter and spreader by the holder's AI index, from 0 (the tables the
-- shared actions read: `off_80EBD80`, `off_80EC944` and `off_80ECC94`, 28
-- entries each). The link navis HeatMan, GroundMan and DustMan (AI indices
-- 1, 9 and 10) hold their own.

local navi_arms = {}

navi_arms.cannon = { 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0 }
navi_arms.air_shooter = { 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0 }
navi_arms.spreader = { 0, 2, 0, 0, 0, 0, 0, 0, 0, 2, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0 }

return navi_arms
