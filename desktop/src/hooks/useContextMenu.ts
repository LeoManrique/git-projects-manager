import { useState, useRef, useEffect, useCallback } from 'react';

export interface MenuPosition {
  top: number;
  left: number;
}

export interface UseContextMenuOptions {
  menuWidth?: number;
  offsetY?: number;
}

export interface UseContextMenuReturn {
  isOpen: boolean;
  position: MenuPosition | null;
  buttonRef: React.RefObject<HTMLButtonElement>;
  menuRef: React.RefObject<HTMLDivElement>;
  open: () => void;
  close: () => void;
  toggle: () => void;
}

export function useContextMenu(options: UseContextMenuOptions = {}): UseContextMenuReturn {
  const { menuWidth = 140, offsetY = 4 } = options;

  const [isOpen, setIsOpen] = useState(false);
  const [position, setPosition] = useState<MenuPosition | null>(null);
  const buttonRef = useRef<HTMLButtonElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);

  // Close menu when clicking outside
  useEffect(() => {
    function handleClickOutside(event: MouseEvent) {
      const target = event.target as Node;
      const isOutsideMenu = menuRef.current && !menuRef.current.contains(target);
      const isOutsideButton = buttonRef.current && !buttonRef.current.contains(target);

      if (isOutsideMenu && isOutsideButton) {
        setIsOpen(false);
      }
    }

    if (isOpen) {
      document.addEventListener('mousedown', handleClickOutside);
      return () => document.removeEventListener('mousedown', handleClickOutside);
    }
  }, [isOpen]);

  const open = useCallback(() => {
    if (buttonRef.current) {
      const rect = buttonRef.current.getBoundingClientRect();
      setPosition({
        top: rect.bottom + offsetY,
        left: rect.right - menuWidth,
      });
    }
    setIsOpen(true);
  }, [menuWidth, offsetY]);

  const close = useCallback(() => {
    setIsOpen(false);
  }, []);

  const toggle = useCallback(() => {
    if (isOpen) {
      close();
    } else {
      open();
    }
  }, [isOpen, open, close]);

  return {
    isOpen,
    position,
    buttonRef,
    menuRef,
    open,
    close,
    toggle,
  };
}
