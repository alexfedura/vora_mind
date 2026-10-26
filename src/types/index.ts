export interface TaskFlowItem {
  id: string;
  title: string;
  completed: boolean;
  position: number;
  createdAt: number;
  updatedAt: number;
}

export interface JournalEntry {
  date: string;
  content: string;
  createdAt: number;
  updatedAt: number;
}

export interface QuestionItem {
  id: string;
  title: string;
  description: string;
  createdAt: number;
  updatedAt: number;
}

export interface CustomFontSetting {
  id: string;
  name: string;
  dataUrl: string;
}
