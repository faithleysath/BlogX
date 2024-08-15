document.addEventListener('DOMContentLoaded', function() {
    // 遍历所有图片，设置referrerpolicy="no-referrer"
    for (const img of document.getElementsByTagName('img')) {
        img.referrerPolicy = 'no-referrer';
    }
    // 将站外链接在新标签页打开，通过修改 target 属性
    for (const a of document.getElementsByTagName('a')) {
        if (a.href.startsWith('http') && !a.href.startsWith(location.origin)) {
            a.target = '_blank';
            a.rel = 'noopener noreferrer';
        }
    }

    //在<div class="codehilite">中添加<a class="copyicon"><i class="fa-solid fa-clipboard"></i></a>
    for (const codehilite of document.getElementsByClassName('codehilite')) {
        const copyIcon = document.createElement('a');
        copyIcon.classList.add('copyicon');
        copyIcon.innerHTML = '<i class="fa-solid fa-clipboard"></i>';
        codehilite.insertBefore(copyIcon, codehilite.firstChild);
    }
    var clipboard = new ClipboardJS('.copyicon', {
        target: function (trigger) {
            return trigger.nextElementSibling.querySelector('code');
        }
    });
    clipboard.on('success', function (e) {
        console.log(e);
        toastr.success('复制成功', '', {
            "closeButton": false,
            "debug": false,
            "newestOnTop": true,
            "progressBar": true,
            "positionClass": "toast-top-right",
            "preventDuplicates": false,
            "onclick": null,
            "showDuration": "300",
            "hideDuration": "1000",
            "timeOut": "2700",  // 改短时间
            "extendedTimeOut": "1000",  // 改短时间
            "showEasing": "swing",
            "hideEasing": "linear",
            "showMethod": "fadeIn",
            "hideMethod": "fadeOut"
        });
        e.clearSelection();  // 清除文本的选中状态
        // Add hover effect class
        var copyIcon = e.trigger;
        copyIcon.classList.add('hover-effect');

        // Remove hover effect class after 0.3 seconds
        setTimeout(function() {
            copyIcon.classList.remove('hover-effect');
        }, 500);
    });
    clipboard.on('error', function (e) {
        console.log(e);
        toastr.error('复制失败', '', {
            "closeButton": false,
            "debug": false,
            "newestOnTop": true,
            "progressBar": true,
            "positionClass": "toast-top-right",
            "preventDuplicates": false,
            "onclick": null,
            "showDuration": "300",
            "hideDuration": "1000",
            "timeOut": "2700",  // 改短时间
            "extendedTimeOut": "1000",  // 改短时间
            "showEasing": "swing",
            "hideEasing": "linear",
            "showMethod": "fadeIn",
            "hideMethod": "fadeOut"
        });
        e.clearSelection();  // 清除文本的选中状态
    });
});