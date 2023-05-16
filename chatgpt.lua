local M = {}

local gpt_cmd = os.getenv("SHELLBOT")
-- local gpt_cmd = "cat"
local ns_vimbot = vim.api.nvim_create_namespace("vimbot")


local roles = {
  USER = "🧑 " .. os.getenv('USER'),
  ASSISSTANT = "🤖 vimbot",
}

local buffer_sync_cursor = {}
function ChatGPTCancelCursorSync()
  local bufnr = vim.api.nvim_get_current_buf()
  buffer_sync_cursor[bufnr] = false
  vim.api.nvim_buf_del_keymap(bufnr, 'n', '<Enter>')
  vim.api.nvim_buf_del_keymap(bufnr, 'n', '<Space>')
end

local function add_transcript_header(winnr, bufnr, role, line_num)
  local line = ((line_num ~= nil) and line_num) or vim.api.nvim_buf_line_count(bufnr)
  vim.api.nvim_buf_set_lines(bufnr, line, line + 1, false, { roles[role] })
  local eid = vim.api.nvim_buf_set_extmark(bufnr, ns_vimbot, line, 0, {})

  local mark_data = vim.api.nvim_buf_get_var(bufnr, 'transcript_marks')
  mark_data[eid] = role
  if role == "USER" and buffer_sync_cursor[bufnr] then
    vim.schedule(function()
      local is_current = winnr == vim.api.nvim_get_current_win()
      vim.api.nvim_win_call(winnr, function()
        vim.cmd("normal! Go")
        if is_current then
          vim.cmd('startinsert!')
        end
      end)
    end)
  end
  return line
end

function ChatGPTSubmit()
  vim.cmd("normal! Go")
  local winnr = vim.api.nvim_get_current_win()
  local bufnr = vim.api.nvim_get_current_buf()
  buffer_sync_cursor[bufnr] = true
  local function receive_stream(_, data, _)
    if #data > 1 or data[1] ~= '' then
      local current_line = vim.api.nvim_buf_line_count(bufnr)
      local col = #vim.api.nvim_buf_get_lines(bufnr, current_line - 1, current_line, false)[1]

      current_line = current_line - 1
      -- print("data " .. current_line .. "," .. col)

      -- - {data}	    Raw data (|readfile()|-style list of strings) read from
      -- the channel. EOF is a single-item list: `['']`. First and
      -- last items may be partial lines! |channel-lines|
      vim.api.nvim_buf_set_option(bufnr, 'modifiable', true)
      for i, new_text in ipairs(data) do
        -- new_text = "[" .. new_text .. "]"
        -- print(i .. ": " .. new_text .. " :" .. current_line .."," .. col .. "|" .. #new_text)
        if i == 1 then
          if #new_text > 0 then
            vim.api.nvim_buf_set_text(bufnr, current_line, col, current_line, col, { new_text })
            col = col + #new_text
          end
        else
          current_line = current_line + 1
          vim.api.nvim_buf_set_lines(bufnr, current_line, current_line, false, { new_text })
          col = #new_text
        end
      end
      if buffer_sync_cursor[bufnr] then
        vim.schedule(function()
          vim.api.nvim_win_call(winnr, function()
            vim.cmd("normal! G$")
          end)
        end)
      end
    end
  end

  local function stream_done()
    vim.api.nvim_buf_set_option(bufnr, 'modifiable', true)
    add_transcript_header(winnr, bufnr, "USER")
  end

  local function get_transcript(separator)
    local lines = vim.api.nvim_buf_get_lines(bufnr, 0, -1, false)

    local mark_data = vim.api.nvim_buf_get_var(bufnr, 'transcript_marks')
    -- Replace marked lines with text from the marks_map
    for extmark_id, role in pairs(mark_data) do
      local mark_pos = vim.api.nvim_buf_get_extmark_by_id(bufnr, ns_vimbot, extmark_id, {})
      local line_num = mark_pos[1]

      if line_num ~= nil then
        lines[line_num + 1] = separator .. role .. separator
      end
    end
    return lines
  end

  local job_id = vim.fn.jobstart(gpt_cmd, {
    on_stdout = receive_stream,
    on_exit = stream_done
  })

  if job_id > 0 then
    local transcript = get_transcript("===")
    for _, line in ipairs(transcript) do
      vim.fn.chansend(job_id, line .. "\n")
      -- print(line)
    end
    local line = add_transcript_header(winnr, bufnr, "ASSISSTANT")
    vim.api.nvim_buf_set_lines(bufnr, line + 1, line + 1, false, { "" })
    vim.api.nvim_buf_set_option(bufnr, 'modifiable', false)
    vim.fn.chanclose(job_id, "stdin")
    vim.api.nvim_command('stopinsert')
    vim.api.nvim_buf_set_keymap(bufnr, 'n', '<Enter>',
      ':lua ChatGPTCancelCursorSync()<cr>', { noremap = true, silent = true })
    vim.api.nvim_buf_set_keymap(bufnr, 'n', '<Space>',
      ':lua ChatGPTCancelCursorSync()<cr>', { noremap = true, silent = true })
  else
    print("Failed to start command")
  end
  if job_id == -1 then
    vim.api.nvim_echo({ { "Failed to start the command", "ErrorMsg" } }, true, {})
  end
end

function ChatGPTNewBuf()
  vim.cmd("enew")
  ChatGPTNew()
end

function ChatGPTNew()
  vim.cmd("vertical resize 60")
  local winnr = vim.api.nvim_get_current_win()
  local bufnr = vim.api.nvim_get_current_buf()
  vim.api.nvim_buf_set_var(bufnr, 'transcript_marks', {})
  buffer_sync_cursor[bufnr] = true
  vim.wo.breakindent = true
  vim.wo.wrap = true
  vim.wo.linebreak = true
  vim.api.nvim_buf_set_option(bufnr, 'buftype', 'nofile')
  vim.api.nvim_buf_set_option(bufnr, 'buflisted', false)
  vim.api.nvim_buf_set_option(bufnr, 'bufhidden', 'hide')
  vim.api.nvim_buf_set_option(bufnr, 'swapfile', false)
  add_transcript_header(winnr, bufnr, "USER", 0)
  local modes = { 'n', 'i' }
  for _, mode in ipairs(modes) do
    vim.api.nvim_buf_set_keymap(bufnr, mode, '<C-Enter>', '<ESC>:lua ChatGPTSubmit()<CR>',
      { noremap = true, silent = true })
    vim.api.nvim_buf_set_keymap(bufnr, mode, '<C-o>', '<ESC>:lua ChatGPTNewBuf()<CR>',
      { noremap = true, silent = true })
  end
end

function M.chatgpt()
  vim.cmd("botright vnew")
  vim.cmd("set winfixwidth")
  ChatGPTNew()
end

return M
