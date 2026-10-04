--!strict
-- BN5's NaviCust board as the engine reads it (rule section `navicust`): what a setup's programs may cover (nettai-match's
-- checks, the editor).

local board = require("./board")

return { boards = board.boards, command_line = board.COMMAND_LINE }
