--!strict
-- What a navi holds in its own look: the animation of a chip's cannon, air shooter and spreader by the holder's AI
-- index, from 0 (the tables EXE5's actions read: the cannons' 0x080EC300, AirShot's 0x080EC9B8 and the Spreaders'
-- 0x080EC5B8, 24 entries each). The team navis NapalmMan (the cannon alone), MagnetMan and KnightMan (AI indices 4, 5
-- and 11) hold their own.

local navi_arms = {}

navi_arms.cannon = { 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0 }
navi_arms.air_shooter = { 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0 }
navi_arms.spreader = { 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0 }

return navi_arms
